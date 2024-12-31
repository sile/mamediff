use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use orfail::OrFail;

use crate::{
    diff::{ChunkDiff, Diff, FileDiff, LineDiff},
    git::Git,
    terminal::{Canvas, Position, Terminal, Text},
};

// const COLLAPSED_MARK: &str = "…";
const COLLAPSED_MARK: &str = "...";

#[derive(Debug)]
pub struct App {
    terminal: Terminal,
    exit: bool,
    git: Git,
    widgets: Vec<DiffWidget>,
    cursor: Cursor,
    show_legend: bool,
}

impl App {
    pub fn new() -> orfail::Result<Self> {
        let git = Git::new();
        let terminal = Terminal::new().or_fail()?;
        Ok(Self {
            terminal,
            exit: false,
            git,
            // TODO: untrack files
            widgets: vec![DiffWidget::new(false), DiffWidget::new(true)],
            cursor: Cursor::new(),
            show_legend: true,
        })
    }

    pub fn run(mut self) -> orfail::Result<()> {
        self.reload_diff().or_fail()?;

        while !self.exit {
            let event = self.terminal.next_event().or_fail()?;
            self.handle_event(event).or_fail()?;
        }
        Ok(())
    }

    fn render(&mut self) -> orfail::Result<()> {
        let mut canvas = Canvas::new();
        for widget in &mut self.widgets {
            widget.render(&mut canvas, &self.cursor).or_fail()?;
        }
        self.render_legend(&mut canvas).or_fail()?;
        self.terminal.render(canvas).or_fail()?;
        Ok(())
    }

    fn is_togglable(&self) -> bool {
        self.widgets.iter().any(|w| w.is_togglable(&self.cursor))
    }

    fn can_stage(&self) -> bool {
        self.widgets.iter().any(|w| w.can_stage(&self.cursor))
    }

    fn can_unstage(&self) -> bool {
        self.widgets.iter().any(|w| w.can_unstage(&self.cursor))
    }

    fn render_legend(&mut self, canvas: &mut Canvas) -> orfail::Result<()> {
        let mut tmp = Canvas::new();
        let cols = if self.show_legend {
            tmp.draw_textl(Text::new("|                  ").or_fail()?);
            tmp.draw_textl(Text::new("| (q)uit [ESC, C-c]").or_fail()?);
            tmp.draw_textl(Text::new("| (r)load          ").or_fail()?);
            if self.is_togglable() {
                tmp.draw_textl(Text::new("| (t)oggle    [TAB]").or_fail()?);
            }
            if self.can_stage() {
                tmp.draw_textl(Text::new("| (s)tage          ").or_fail()?);
            }
            if self.can_stage() {
                tmp.draw_textl(Text::new("| (D)iscard        ").or_fail()?);
            }
            if self.can_unstage() {
                tmp.draw_textl(Text::new("| (u)nstage        ").or_fail()?);
            }
            tmp.draw_textl(Text::new("|                  ").or_fail()?);
            tmp.draw_textl(Text::new("+- (h)ide ---------").or_fail()?);
            20
        } else {
            tmp.draw_textl(Text::new("|          ").or_fail()?);
            tmp.draw_textl(Text::new("+- s(h)ow -").or_fail()?);
            11
        };
        tmp.draw_newline();

        canvas.draw_canvas(
            Position::new(0, self.terminal.size().cols.saturating_sub(cols)),
            tmp,
        );
        Ok(())
    }

    fn handle_event(&mut self, event: Event) -> orfail::Result<()> {
        match event {
            Event::FocusGained => Ok(()),
            Event::FocusLost => Ok(()),
            Event::Key(event) => self.handle_key_event(event).or_fail(),
            Event::Mouse(_) => Ok(()),
            Event::Paste(_) => Ok(()),
            Event::Resize(_, _) => {
                self.terminal.on_resized().or_fail()?;
                self.render().or_fail()
            }
        }
    }

    fn handle_key_event(&mut self, event: KeyEvent) -> orfail::Result<()> {
        if event.kind != KeyEventKind::Press {
            return Ok(());
        }

        match event.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.exit = true;
            }
            KeyCode::Char('c') if event.modifiers.contains(KeyModifiers::CONTROL) => {
                self.exit = true;
            }
            KeyCode::Char('r') => {
                self.reload_diff().or_fail()?;
            }
            KeyCode::Char('u') => {
                todo!()
            }
            KeyCode::Char('s') => {
                self.handle_stage().or_fail()?;
            }
            KeyCode::Char('D') => {
                todo!()
            }
            KeyCode::Char('h') => {
                self.show_legend = !self.show_legend;
                self.render().or_fail()?;
            }
            KeyCode::Char('p') if event.modifiers.contains(KeyModifiers::CONTROL) => {
                self.handle_up().or_fail()?;
                self.render().or_fail()?;
            }
            KeyCode::Up => {
                self.handle_up().or_fail()?;
                self.render().or_fail()?;
            }
            KeyCode::Char('n') if event.modifiers.contains(KeyModifiers::CONTROL) => {
                self.handle_down().or_fail()?;
                self.render().or_fail()?;
            }
            KeyCode::Down => {
                self.handle_down().or_fail()?;
                self.render().or_fail()?;
            }
            KeyCode::Char('f') if event.modifiers.contains(KeyModifiers::CONTROL) => {
                self.handle_right().or_fail()?;
                self.render().or_fail()?;
            }
            KeyCode::Right => {
                self.handle_right().or_fail()?;
                self.render().or_fail()?;
            }
            KeyCode::Char('b') if event.modifiers.contains(KeyModifiers::CONTROL) => {
                self.handle_left().or_fail()?;
                self.render().or_fail()?;
            }
            KeyCode::Left => {
                self.handle_left().or_fail()?;
                self.render().or_fail()?;
            }
            KeyCode::Char('t') | KeyCode::Tab => {
                self.handle_tab().or_fail()?;
                self.render().or_fail()?;
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_tab(&mut self) -> orfail::Result<()> {
        for widget in &mut self.widgets {
            widget.toggle(&self.cursor).or_fail()?;
        }
        Ok(())
    }

    fn handle_up(&mut self) -> orfail::Result<()> {
        for widget in &mut self.widgets {
            widget.handle_up(&mut self.cursor).or_fail()?;
        }
        Ok(())
    }

    fn handle_down(&mut self) -> orfail::Result<()> {
        for widget in &mut self.widgets {
            widget.handle_down(&mut self.cursor).or_fail()?;
        }
        Ok(())
    }

    fn handle_right(&mut self) -> orfail::Result<()> {
        for widget in &mut self.widgets {
            widget.handle_right(&mut self.cursor).or_fail()?;
        }
        Ok(())
    }

    fn handle_left(&mut self) -> orfail::Result<()> {
        for widget in &mut self.widgets {
            widget.handle_left(&mut self.cursor).or_fail()?;
        }
        Ok(())
    }

    fn reload_diff(&mut self) -> orfail::Result<()> {
        for widget in &mut self.widgets {
            widget.reload(&self.git).or_fail()?;
        }
        self.render().or_fail()?;
        Ok(())
    }

    fn handle_stage(&mut self) -> orfail::Result<()> {
        false.or_fail()
    }
}

#[derive(Debug)]
pub struct DiffWidget {
    widget_path: WidgetPath,
    staged: bool,
    diff: Diff,
    children: Vec<FileDiffWidget>,
    expanded: bool,
}

impl DiffWidget {
    pub fn new(staged: bool) -> Self {
        let index = if staged { 1 } else { 0 };
        Self {
            widget_path: WidgetPath::new(vec![index]),
            staged,
            diff: Diff::default(),
            children: Vec::new(),
            expanded: true,
        }
    }

    pub fn is_togglable(&self, cursor: &Cursor) -> bool {
        if self.widget_path.path == cursor.path {
            !self.children.is_empty()
        } else if cursor.path.starts_with(&self.widget_path.path) {
            self.children.iter().any(|w| w.is_togglable(cursor))
        } else {
            false
        }
    }

    pub fn can_stage(&self, cursor: &Cursor) -> bool {
        if self.staged {
            false
        } else if self.widget_path.path == cursor.path {
            !self.children.is_empty()
        } else if cursor.path.starts_with(&self.widget_path.path) {
            self.children.iter().any(|w| w.can_stage(cursor))
        } else {
            false
        }
    }

    pub fn can_unstage(&self, cursor: &Cursor) -> bool {
        if !self.staged {
            false
        } else if self.widget_path.path == cursor.path {
            !self.children.is_empty()
        } else if cursor.path.starts_with(&self.widget_path.path) {
            self.children.iter().any(|w| w.can_unstage(cursor))
        } else {
            false
        }
    }

    pub fn toggle(&mut self, cursor: &Cursor) -> orfail::Result<()> {
        (cursor.path.len() >= Self::LEVEL).or_fail()?;

        if self.children.is_empty() || cursor.path[Self::LEVEL - 1] != self.widget_path.last_index()
        {
            return Ok(());
        }

        if cursor.path.len() == Self::LEVEL {
            self.expanded = !self.expanded;
        } else {
            for child in &mut self.children {
                child.toggle(cursor).or_fail()?;
            }
        }

        Ok(())
    }

    pub fn handle_left(&mut self, cursor: &mut Cursor) -> orfail::Result<()> {
        (cursor.path.len() >= 1).or_fail()?;

        if cursor.path[0] == self.widget_path.last_index() && cursor.path.len() > 1 {
            cursor.path.pop();
        }

        Ok(())
    }

    // TODO: factor out
    pub const LEVEL: usize = 1;

    pub fn handle_right(&mut self, cursor: &mut Cursor) -> orfail::Result<()> {
        (cursor.path.len() >= Self::LEVEL).or_fail()?;

        if self.children.is_empty() || cursor.path[Self::LEVEL - 1] != self.widget_path.last_index()
        {
            return Ok(());
        }

        if cursor.path.len() == Self::LEVEL {
            cursor.path.push(0);
            self.expanded = true;
        } else {
            for child in &mut self.children {
                child.handle_right(cursor).or_fail()?;
            }
        }

        Ok(())
    }

    pub fn handle_down(&mut self, cursor: &mut Cursor) -> orfail::Result<()> {
        (cursor.path.len() >= Self::LEVEL).or_fail()?;

        if cursor.path.len() == Self::LEVEL {
            if self.widget_path.last_index() == cursor.path[Self::LEVEL - 1] + 1 {
                cursor.path[Self::LEVEL - 1] += 1;
            }
        } else if self.widget_path.last_index() == cursor.path[Self::LEVEL - 1] {
            for child in self.children.iter_mut().rev() {
                child.handle_down(cursor).or_fail()?;
            }
        }

        // TODO: next higher level item if the last item of the level is reached.

        Ok(())
    }

    pub fn handle_up(&mut self, cursor: &mut Cursor) -> orfail::Result<()> {
        (cursor.path.len() >= Self::LEVEL).or_fail()?;

        if cursor.path.len() == Self::LEVEL {
            if Some(self.widget_path.last_index()) == cursor.path[Self::LEVEL - 1].checked_sub(1) {
                cursor.path[Self::LEVEL - 1] -= 1;
            }
        } else if self.widget_path.last_index() == cursor.path[Self::LEVEL - 1] {
            for child in &mut self.children {
                child.handle_up(cursor).or_fail()?;
            }
        }

        Ok(())
    }

    pub fn reload(&mut self, git: &Git) -> orfail::Result<()> {
        // TODO: Execute in parallel
        self.diff = if self.staged {
            git.diff_cached().or_fail()?
        } else {
            git.diff().or_fail()?
        };

        // TODO: merge old state (e.g., focused)
        self.children.clear();
        for (i, file) in self.diff.files.iter().enumerate() {
            self.children
                .push(FileDiffWidget::new(file, self.widget_path.join(i)));
        }

        Ok(())
    }

    pub fn render(&self, canvas: &mut Canvas, cursor: &Cursor) -> orfail::Result<()> {
        canvas.draw_text(
            Text::new(&format!(
                "{}{} {} changes ({} files){}",
                if self.widget_path.path == cursor.path {
                    ">"
                } else {
                    " "
                },
                if cursor.path.len() == 1 { "|" } else { " " },
                if self.staged { "Staged" } else { "Unstaged" },
                self.diff.len(),
                if self.expanded { "" } else { COLLAPSED_MARK }
            ))
            .or_fail()?,
        );
        canvas.draw_newline();

        if self.expanded {
            for (child, diff) in self.children.iter().zip(self.diff.files.iter()) {
                child.render(canvas, diff, cursor).or_fail()?;
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct FileDiffWidget {
    pub widget_path: WidgetPath,
    pub children: Vec<ChunkDiffWidget>,
    pub expanded: bool,
}

impl FileDiffWidget {
    pub fn new(diff: &FileDiff, widget_path: WidgetPath) -> Self {
        let children = diff
            .chunks()
            .enumerate()
            .map(|(i, c)| ChunkDiffWidget::new(c, widget_path.join(i)))
            .collect();
        Self {
            widget_path,
            children,
            expanded: false,
        }
    }

    pub fn render(
        &self,
        canvas: &mut Canvas,
        diff: &FileDiff,
        cursor: &Cursor,
    ) -> orfail::Result<()> {
        // TODO: rename handling
        canvas.draw_text(
            Text::new(&format!(
                "{}{} modified {} ({} chunks){}",
                match cursor.path.len() {
                    1 => " .",
                    _ => "  ",
                },
                if self.widget_path.path == cursor.path {
                    ">|"
                } else if cursor.path.len() == 2 {
                    " |"
                } else {
                    "  "
                },
                diff.path().display(),
                self.children.len(),
                if self.expanded { "" } else { COLLAPSED_MARK }
            ))
            .or_fail()?,
        );
        canvas.draw_newline();

        if self.expanded {
            for (child, chunk) in self.children.iter().zip(diff.chunks()) {
                child.render(canvas, chunk, cursor).or_fail()?;
            }
        }

        Ok(())
    }

    pub fn toggle(&mut self, cursor: &Cursor) -> orfail::Result<()> {
        (cursor.path.len() >= Self::LEVEL).or_fail()?;

        if self.children.is_empty() || cursor.path[Self::LEVEL - 1] != self.widget_path.last_index()
        {
            return Ok(());
        }

        if cursor.path.len() == Self::LEVEL {
            self.expanded = !self.expanded;
        } else {
            for child in &mut self.children {
                child.toggle(cursor).or_fail()?;
            }
        }

        Ok(())
    }

    pub fn is_togglable(&self, cursor: &Cursor) -> bool {
        if self.widget_path.path == cursor.path {
            !self.children.is_empty()
        } else if cursor.path.starts_with(&self.widget_path.path) {
            self.children.iter().any(|w| w.is_togglable(cursor))
        } else {
            false
        }
    }

    pub fn can_stage(&self, cursor: &Cursor) -> bool {
        if self.widget_path.path == cursor.path {
            !self.children.is_empty()
        } else if cursor.path.starts_with(&self.widget_path.path) {
            self.children.iter().any(|w| w.can_stage(cursor))
        } else {
            false
        }
    }

    pub fn can_unstage(&self, cursor: &Cursor) -> bool {
        if self.widget_path.path == cursor.path {
            !self.children.is_empty()
        } else if cursor.path.starts_with(&self.widget_path.path) {
            self.children.iter().any(|w| w.can_unstage(cursor))
        } else {
            false
        }
    }

    pub const LEVEL: usize = 2;

    pub fn handle_right(&mut self, cursor: &mut Cursor) -> orfail::Result<()> {
        (cursor.path.len() >= Self::LEVEL).or_fail()?;

        if self.children.is_empty() || cursor.path[Self::LEVEL - 1] != self.widget_path.last_index()
        {
            return Ok(());
        }

        if cursor.path.len() == Self::LEVEL {
            cursor.path.push(0);
            self.expanded = true;
        } else {
            for child in &mut self.children {
                child.handle_right(cursor).or_fail()?;
            }
        }

        Ok(())
    }

    pub fn handle_down(&mut self, cursor: &mut Cursor) -> orfail::Result<()> {
        (cursor.path.len() >= Self::LEVEL).or_fail()?;

        if cursor.path.len() == Self::LEVEL {
            if self.widget_path.last_index() == cursor.path[Self::LEVEL - 1] + 1 {
                cursor.path[Self::LEVEL - 1] += 1;
            }
        } else if self.widget_path.last_index() == cursor.path[Self::LEVEL - 1] {
            for child in self.children.iter_mut().rev() {
                child.handle_down(cursor).or_fail()?;
            }
        }

        // TODO: next higher level item if the last item of the level is reached.

        Ok(())
    }

    pub fn handle_up(&mut self, cursor: &mut Cursor) -> orfail::Result<()> {
        (cursor.path.len() >= Self::LEVEL).or_fail()?;

        if cursor.path.len() == Self::LEVEL {
            if Some(self.widget_path.last_index()) == cursor.path[Self::LEVEL - 1].checked_sub(1) {
                cursor.path[Self::LEVEL - 1] -= 1;
            }
        } else if self.widget_path.last_index() == cursor.path[Self::LEVEL - 1] {
            for child in &mut self.children {
                child.handle_up(cursor).or_fail()?;
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct ChunkDiffWidget {
    pub widget_path: WidgetPath,
    pub children: Vec<LineDiffWidget>,
    pub expanded: bool,
}

impl ChunkDiffWidget {
    pub fn new(diff: &ChunkDiff, widget_path: WidgetPath) -> Self {
        let children = diff
            .lines
            .iter()
            .enumerate()
            .map(|(i, l)| LineDiffWidget::new(l, widget_path.join(i)))
            .collect();
        Self {
            widget_path,
            children,
            expanded: true,
        }
    }

    pub fn render(
        &self,
        canvas: &mut Canvas,
        diff: &ChunkDiff,
        cursor: &Cursor,
    ) -> orfail::Result<()> {
        canvas.draw_text(
            Text::new(&format!(
                "{}{} {}{}",
                match cursor.path.len() {
                    1 => " .  ",
                    2 => "   .",
                    _ => "    ",
                },
                if self.widget_path.path == cursor.path {
                    ">|"
                } else if cursor.path.len() == 3 {
                    " |"
                } else {
                    "  "
                },
                diff.head_line(),
                if self.expanded { "" } else { COLLAPSED_MARK }
            ))
            .or_fail()?,
        );
        canvas.draw_newline();

        if self.expanded {
            for (child, line) in self.children.iter().zip(diff.lines.iter()) {
                child.render(canvas, line, cursor).or_fail()?;
            }
        }

        Ok(())
    }

    pub fn toggle(&mut self, cursor: &Cursor) -> orfail::Result<()> {
        (cursor.path.len() >= Self::LEVEL).or_fail()?;

        if self.children.is_empty()
            || cursor.path.len() > Self::LEVEL
            || cursor.path[Self::LEVEL - 1] != self.widget_path.last_index()
        {
            return Ok(());
        }

        self.expanded = !self.expanded;

        Ok(())
    }

    pub const LEVEL: usize = 3;

    pub fn can_stage(&self, cursor: &Cursor) -> bool {
        if self.widget_path.path == cursor.path {
            !self.children.is_empty()
        } else if cursor.path.starts_with(&self.widget_path.path) {
            self.children.iter().any(|w| w.can_stage(cursor))
        } else {
            false
        }
    }

    pub fn can_unstage(&self, cursor: &Cursor) -> bool {
        if self.widget_path.path == cursor.path {
            !self.children.is_empty()
        } else if cursor.path.starts_with(&self.widget_path.path) {
            self.children.iter().any(|w| w.can_unstage(cursor))
        } else {
            false
        }
    }

    pub fn is_togglable(&self, cursor: &Cursor) -> bool {
        if self.widget_path.path == cursor.path {
            !self.children.is_empty()
        } else if cursor.path.starts_with(&self.widget_path.path) {
            self.children.iter().any(|w| w.is_togglable(cursor))
        } else {
            false
        }
    }

    pub fn handle_right(&mut self, cursor: &mut Cursor) -> orfail::Result<()> {
        (cursor.path.len() >= Self::LEVEL).or_fail()?;

        if self.children.is_empty() || cursor.path[Self::LEVEL - 1] != self.widget_path.last_index()
        {
            return Ok(());
        }

        if cursor.path.len() == Self::LEVEL {
            cursor.path.push(0);
            self.expanded = true;
        } else {
            for child in self.children.iter_mut() {
                child.handle_right(cursor).or_fail()?;
            }
        }

        Ok(())
    }

    pub fn handle_down(&mut self, cursor: &mut Cursor) -> orfail::Result<()> {
        (cursor.path.len() >= Self::LEVEL).or_fail()?;

        if cursor.path.len() == Self::LEVEL {
            if self.widget_path.last_index() == cursor.path[Self::LEVEL - 1] + 1 {
                cursor.path[Self::LEVEL - 1] += 1;
            }
        } else if self.widget_path.last_index() == cursor.path[Self::LEVEL - 1] {
            for child in self.children.iter_mut().rev() {
                child.handle_down(cursor).or_fail()?;
            }
        }

        Ok(())
    }

    pub fn handle_up(&mut self, cursor: &mut Cursor) -> orfail::Result<()> {
        (cursor.path.len() >= Self::LEVEL).or_fail()?;

        if cursor.path.len() == Self::LEVEL {
            if Some(self.widget_path.last_index()) == cursor.path[Self::LEVEL - 1].checked_sub(1) {
                cursor.path[Self::LEVEL - 1] -= 1;
            }
        } else if self.widget_path.last_index() == cursor.path[Self::LEVEL - 1] {
            for child in self.children.iter_mut() {
                child.handle_up(cursor).or_fail()?;
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct WidgetPath {
    pub path: Vec<usize>,
}

impl WidgetPath {
    pub fn new(path: Vec<usize>) -> Self {
        Self { path }
    }

    pub fn last_index(&self) -> usize {
        // TODO:
        self.path[self.path.len() - 1]
    }

    pub fn join(&self, index: usize) -> Self {
        let mut path = self.path.clone();
        path.push(index);
        Self::new(path)
    }
}

#[derive(Debug)]
pub struct Cursor {
    pub path: Vec<usize>,
}

impl Cursor {
    pub fn new() -> Self {
        Self { path: vec![0] }
    }
}

#[derive(Debug)]
pub struct LineDiffWidget {
    pub widget_path: WidgetPath,
    pub has_diff: bool,
}

impl LineDiffWidget {
    pub fn new(diff: &LineDiff, widget_path: WidgetPath) -> Self {
        Self {
            widget_path,
            has_diff: !matches!(diff, LineDiff::Both(_)),
        }
    }

    pub fn render(
        &self,
        canvas: &mut Canvas,
        diff: &LineDiff,
        cursor: &Cursor,
    ) -> orfail::Result<()> {
        canvas.draw_text(
            Text::new(&format!(
                "{}{} {}",
                match cursor.path.len() {
                    1 => " .    ",
                    2 => "   .  ",
                    3 => "     .",
                    _ => "      ",
                },
                if self.widget_path.path == cursor.path {
                    ">|"
                } else if cursor.path.len() == 4 {
                    " |"
                } else {
                    "  "
                },
                // if self.widget_path.path == cursor.path {
                //     ">"
                // } else {
                //     " "
                // },
                diff
            ))
            .or_fail()?,
        );
        canvas.draw_newline();

        Ok(())
    }

    pub const LEVEL: usize = 4;

    pub fn can_stage(&self, cursor: &Cursor) -> bool {
        if self.widget_path.path == cursor.path {
            self.has_diff
        } else {
            false
        }
    }

    pub fn can_unstage(&self, cursor: &Cursor) -> bool {
        if self.widget_path.path == cursor.path {
            self.has_diff
        } else {
            false
        }
    }

    pub fn is_togglable(&self, _cursor: &Cursor) -> bool {
        false
    }

    pub fn handle_right(&mut self, cursor: &mut Cursor) -> orfail::Result<()> {
        (cursor.path.len() >= Self::LEVEL).or_fail()?;
        Ok(())
    }

    pub fn handle_down(&mut self, cursor: &mut Cursor) -> orfail::Result<()> {
        (cursor.path.len() >= Self::LEVEL).or_fail()?;

        if cursor.path.len() == Self::LEVEL {
            if self.widget_path.last_index() == cursor.path[Self::LEVEL - 1] + 1 {
                cursor.path[Self::LEVEL - 1] += 1;
            }
        }

        Ok(())
    }

    pub fn handle_up(&mut self, cursor: &mut Cursor) -> orfail::Result<()> {
        (cursor.path.len() >= Self::LEVEL).or_fail()?;

        if cursor.path.len() == Self::LEVEL {
            if Some(self.widget_path.last_index()) == cursor.path[Self::LEVEL - 1].checked_sub(1) {
                cursor.path[Self::LEVEL - 1] -= 1;
            }
        }

        Ok(())
    }
}
