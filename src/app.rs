use std::{ops::Range, path::PathBuf};

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
    row_offset: usize,
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
            row_offset: 0,
        })
    }

    pub fn run(mut self) -> orfail::Result<()> {
        self.reload_diff_reset().or_fail()?;

        while !self.exit {
            let event = self.terminal.next_event().or_fail()?;
            self.handle_event(event).or_fail()?;
        }
        Ok(())
    }

    fn full_rows(&self) -> usize {
        self.widgets.iter().map(|w| w.full_rows()).sum()
    }

    fn expand_all(&mut self) {
        for w in &mut self.widgets {
            w.expand_all();
        }
    }

    fn render(&mut self) -> orfail::Result<()> {
        if self.terminal.size().is_empty() {
            return Ok(());
        }

        let cursor_abs_row = self.cursor_abs_row();
        if cursor_abs_row
            .checked_sub(self.row_offset)
            .map_or(true, |p| p >= self.terminal.size().rows)
        {
            let rows = self.terminal.size().rows;
            self.row_offset = cursor_abs_row.saturating_sub(rows / 2);
        }

        let mut canvas = Canvas::new();
        for widget in &mut self.widgets {
            widget.render(&mut canvas, &self.cursor).or_fail()?;
        }

        // TODO: optimize (skip rendering for out of range part)
        canvas.clip(self.row_offset, self.terminal.size().rows);

        self.render_legend(&mut canvas).or_fail()?;
        canvas.clip(0, self.terminal.size().rows); // TODO

        self.terminal.render(canvas).or_fail()?;
        Ok(())
    }

    fn cursor_abs_row(&self) -> usize {
        self.widgets
            .iter()
            .map(|w| w.cursor_abs_row(&self.cursor))
            .sum()
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
            tmp.draw_textl(Text::new("| (q)uit [ESC,C-c]").or_fail()?);
            tmp.draw_textl(Text::new("| (r)eload        ").or_fail()?);

            if self.cursor.path.last() != Some(&0) {
                tmp.draw_textl(Text::new("| (↑)        [C-p]").or_fail()?);
            }
            if self.can_down() {
                tmp.draw_textl(Text::new("| (↓)        [C-n]").or_fail()?);
            }
            if self.cursor.path.len() > 1 {
                tmp.draw_textl(Text::new("| (←)        [C-f]").or_fail()?);
            }
            if self.can_right() {
                tmp.draw_textl(Text::new("| (→)        [C-b]").or_fail()?);
            }
            if self.is_togglable() {
                tmp.draw_textl(Text::new("| (t)oggle   [TAB]").or_fail()?);
            }
            if self.can_stage() {
                tmp.draw_textl(Text::new("| (s)tage         ").or_fail()?);
            }
            if self.can_stage() {
                tmp.draw_textl(Text::new("| (D)iscard       ").or_fail()?);
            }
            if self.can_unstage() {
                tmp.draw_textl(Text::new("| (u)nstage       ").or_fail()?);
            }
            tmp.draw_textl(Text::new("+---- (h)ide -----").or_fail()?);
            19
        } else {
            tmp.draw_textl(Text::new("|   ...    ").or_fail()?);
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
            Event::Mouse(_) => Ok(()), // TODO: Add mouse handling
            Event::Paste(_) => Ok(()),
            Event::Resize(_, _) => {
                self.terminal.on_resized().or_fail()?;
                let cursor_abs_row = self.cursor_abs_row();
                let rows = self.terminal.size().rows;
                self.row_offset = cursor_abs_row.saturating_sub(rows / 2);
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
                self.reload_diff_reset().or_fail()?;
            }
            KeyCode::Char('u') => {
                self.handle_unstage().or_fail()?;
            }
            KeyCode::Char('s') => {
                self.handle_stage().or_fail()?;
            }
            KeyCode::Char('D') => {
                self.handle_discard().or_fail()?;
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

    fn get_children_len(&self) -> usize {
        let i = self.cursor.path[0];
        self.widgets[i].get_children_len(&self.cursor)
    }

    fn handle_up(&mut self) -> orfail::Result<()> {
        // TODO: factor out with can_up()
        let old_cursor = self.cursor.clone();

        for widget in &mut self.widgets {
            widget.handle_up(&mut self.cursor).or_fail()?;
        }

        if old_cursor == self.cursor {
            let level = self.cursor.path.len();

            while self.cursor.path.len() > 1 {
                self.cursor.path.pop();
                let old = self.cursor.clone();
                for widget in &mut self.widgets {
                    widget.handle_up(&mut self.cursor).or_fail()?;
                }
                if old != self.cursor {
                    while self.cursor.path.len() < level {
                        let n = self.get_children_len();
                        self.cursor.path.push(n.checked_sub(1).or_fail()?);
                    }
                    break;
                }
            }
            if !self.is_valid_cursor() {
                self.cursor = old_cursor;
            }
            // TODO: expand cursor position if need
        }

        // TODO: factor out
        let cursor_abs_row = self.cursor_abs_row();
        let current_rows = self
            .widgets
            .iter()
            .find_map(|w| w.current_rows(&self.cursor))
            .or_fail()?;
        let desired_end_row = cursor_abs_row + current_rows + 1;
        if self.row_offset + self.terminal.size().rows < desired_end_row {
            self.row_offset =
                cursor_abs_row.min(desired_end_row.saturating_sub(self.terminal.size().rows));
        }

        Ok(())
    }

    fn handle_down(&mut self) -> orfail::Result<()> {
        // TODO: factor out with can_down()
        let old_cursor = self.cursor.clone();
        for widget in &mut self.widgets {
            widget.handle_down(&mut self.cursor).or_fail()?;
        }

        if old_cursor == self.cursor {
            let level = self.cursor.path.len();

            while self.cursor.path.len() > 1 {
                self.cursor.path.pop();
                let old = self.cursor.clone();
                for widget in &mut self.widgets {
                    widget.handle_down(&mut self.cursor).or_fail()?;
                }
                if old != self.cursor {
                    while self.cursor.path.len() < level {
                        self.cursor.path.push(0);
                    }
                    break;
                }
            }
            if self.cursor.path.len() != level || !self.is_valid_cursor() {
                self.cursor = old_cursor;
            }
            // TODO: expand cursor position if need
        }

        // TODO: factor out
        let cursor_abs_row = self.cursor_abs_row();
        let current_rows = self
            .widgets
            .iter()
            .find_map(|w| w.current_rows(&self.cursor))
            .or_fail()?;
        let desired_end_row = cursor_abs_row + current_rows + 1;
        if self.row_offset + self.terminal.size().rows < desired_end_row {
            self.row_offset =
                cursor_abs_row.min(desired_end_row.saturating_sub(self.terminal.size().rows));
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
        let old_widgets = self.widgets.clone(); // TODO
        for widget in &mut self.widgets {
            widget.reload(&self.git, &old_widgets).or_fail()?;
        }

        while !self.is_valid_cursor() && self.cursor.prev() {}
        // TODO: expand cursor position if need

        self.render().or_fail()?;
        Ok(())
    }

    fn is_valid_cursor(&self) -> bool {
        self.widgets
            .get(self.cursor.path[0])
            .is_some_and(|w| w.is_valid_cursor(&self.cursor))
    }

    fn can_right(&self) -> bool {
        let n = self.cursor.path.len();
        if n == 4 {
            return false;
        }

        // TODO: optimize
        for w in &self.widgets {
            if n == 1 && !w.children.is_empty() {
                return true;
            }

            for c in &w.children {
                if n == 2 && !c.children.is_empty() {
                    return true;
                }

                for c in &c.children {
                    if n == 3 && !c.children.is_empty() {
                        return true;
                    }
                }
            }
        }
        false
    }

    // TODO: remove mut
    fn can_down(&mut self) -> bool {
        // TODO: Allow down key even if the last item if the terminal can scroll down
        let original_cursor = self.cursor.clone();
        let mut valid = false;

        if let Some(x) = self.cursor.path.last_mut() {
            *x += 1;
            valid = self.is_valid_cursor();
        }

        // TODO:
        // if !valid {
        //     self.cursor.path.pop();
        //     // TOD0: while self.cursor.next_sigbling() {
        //     if let Some(x) = self.cursor.path.last_mut() {
        //         *x += 1;
        //         self.cursor.path.push(0);
        //         valid = self.is_valid_cursor();
        //     }
        // }

        self.cursor = original_cursor;
        valid
    }

    fn reload_diff_reset(&mut self) -> orfail::Result<()> {
        let old_widgets = vec![DiffWidget::new(false), DiffWidget::new(true)];
        self.cursor = Cursor::new();
        for widget in &mut self.widgets {
            widget.reload(&self.git, &old_widgets).or_fail()?;
        }
        if self.full_rows() <= self.terminal.size().rows {
            self.expand_all();
        }
        self.render().or_fail()?; // TODO: optimize
        Ok(())
    }

    fn handle_stage(&mut self) -> orfail::Result<()> {
        if self.can_stage() {
            self.widgets[0]
                .handle_stage(&self.git, &self.cursor)
                .or_fail()?;
            self.reload_diff().or_fail()?;
        }
        Ok(())
    }

    fn handle_discard(&mut self) -> orfail::Result<()> {
        // TODO: rename `can_state()`
        if self.can_stage() {
            self.widgets[0]
                .handle_discard(&self.git, &self.cursor)
                .or_fail()?;
            self.reload_diff().or_fail()?;
        }
        Ok(())
    }

    fn handle_unstage(&mut self) -> orfail::Result<()> {
        if self.can_unstage() {
            self.widgets[1]
                .handle_unstage(&self.git, &self.cursor)
                .or_fail()?;
            self.reload_diff().or_fail()?;
        }
        Ok(())
    }
}

// TODO: Add Widget trait
#[derive(Debug, Clone)]
pub struct DiffWidget {
    name: &'static str,
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
            name: if staged { "Staged" } else { "Unstaged" },
            widget_path: WidgetPath::new(vec![index]),
            staged,
            diff: Diff::default(),
            children: Vec::new(),
            expanded: true,
        }
    }

    fn get_children_len(&self, cursor: &Cursor) -> usize {
        if self.widget_path.path == cursor.path {
            self.children.len()
        } else if cursor.path.starts_with(&self.widget_path.path) {
            self.children[cursor.path[Self::LEVEL]].get_children_len(cursor)
        } else {
            // TODO: error?
            0
        }
    }

    fn is_valid_cursor(&self, cursor: &Cursor) -> bool {
        if self.widget_path.path == cursor.path {
            true
        } else if cursor.path.starts_with(&self.widget_path.path) {
            self.children
                .get(cursor.path[Self::LEVEL])
                .is_some_and(|c| c.is_valid_cursor(cursor))
        } else {
            false
        }
    }

    fn handle_stage(&mut self, git: &Git, cursor: &Cursor) -> orfail::Result<()> {
        if !self.can_stage(cursor) {
            return Ok(());
        }

        if cursor.path != self.widget_path.path {
            let i = cursor.path[Self::LEVEL];
            self.children
                .get_mut(i)
                .or_fail()?
                .handle_stage(git, cursor, self.diff.files.get(i).or_fail()?)
                .or_fail()?;
        } else {
            git.stage(&self.diff).or_fail()?;
        }

        Ok(())
    }

    fn handle_discard(&mut self, git: &Git, cursor: &Cursor) -> orfail::Result<()> {
        // TODO: Add comment (it's okay to use can_stage() here)
        if !self.can_stage(cursor) {
            return Ok(());
        }

        if cursor.path != self.widget_path.path {
            let i = cursor.path[Self::LEVEL];
            self.children
                .get_mut(i)
                .or_fail()?
                .handle_discard(git, cursor, self.diff.files.get(i).or_fail()?)
                .or_fail()?;
        } else {
            git.discard(&self.diff).or_fail()?;
        }

        Ok(())
    }

    fn handle_unstage(&mut self, git: &Git, cursor: &Cursor) -> orfail::Result<()> {
        if !self.can_unstage(cursor) {
            return Ok(());
        }

        if cursor.path != self.widget_path.path {
            let i = cursor.path[Self::LEVEL];
            self.children
                .get_mut(i)
                .or_fail()?
                .handle_unstage(git, cursor, self.diff.files.get(i).or_fail()?)
                .or_fail()?;
        } else {
            git.unstage(&self.diff).or_fail()?;
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

    pub fn reload(&mut self, git: &Git, old_widgets: &[DiffWidget]) -> orfail::Result<()> {
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

        self.restore_state(old_widgets);

        Ok(())
    }

    fn restore_state(&mut self, old_widgets: &[DiffWidget]) {
        let i = self.widget_path.path[Self::LEVEL - 1];
        self.expanded = old_widgets[i].expanded;

        for c in &mut self.children {
            let old = old_widgets
                .iter()
                .filter_map(|w| w.children.iter().find(|old| old.name == c.name))
                .collect::<Vec<_>>();
            c.restore_state(&old);
        }
    }

    pub fn rows(&self) -> usize {
        if self.expanded {
            1 + self.children.iter().map(|c| c.rows()).sum::<usize>()
        } else {
            1
        }
    }

    pub fn full_rows(&self) -> usize {
        1 + self.children.iter().map(|c| c.full_rows()).sum::<usize>()
    }

    pub fn expand_all(&mut self) {
        self.expanded = true;
        for c in &mut self.children {
            c.expand_all();
        }
    }

    pub fn current_rows(&self, cursor: &Cursor) -> Option<usize> {
        if self.widget_path.path == cursor.path {
            Some(self.rows())
        } else if cursor.path.starts_with(&self.widget_path.path) {
            let i = cursor.path[Self::LEVEL];
            self.children[i].current_rows(cursor)
        } else {
            None
        }
    }

    pub fn cursor_abs_row(&self, cursor: &Cursor) -> usize {
        match cursor.path[..Self::LEVEL].cmp(&self.widget_path.path) {
            std::cmp::Ordering::Less => 0,
            std::cmp::Ordering::Equal => {
                if cursor.path.len() == Self::LEVEL {
                    0
                } else {
                    1 + self
                        .children
                        .iter()
                        .map(|c| c.cursor_abs_row(cursor))
                        .sum::<usize>()
                }
            }
            std::cmp::Ordering::Greater => self.rows(),
        }
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
                self.name,
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

#[derive(Debug, Clone)]
pub struct FileDiffWidget {
    pub name: PathBuf,
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
            name: diff.path().clone(),
            widget_path,
            children,
            expanded: false,
        }
    }

    fn restore_state(&mut self, old: &[&Self]) {
        if old.is_empty() {
            return;
        }

        self.expanded = old.iter().any(|w| w.expanded);

        for c in &mut self.children {
            let old = old
                .iter()
                .filter_map(|w| w.children.iter().find(|old| old.intersect(c)))
                .collect::<Vec<_>>();
            c.restore_state(&old);
        }
    }

    fn get_children_len(&self, cursor: &Cursor) -> usize {
        if self.widget_path.path == cursor.path {
            self.children.len()
        } else if cursor.path.starts_with(&self.widget_path.path) {
            self.children[cursor.path[Self::LEVEL]].get_children_len(cursor)
        } else {
            // TODO: error?
            0
        }
    }

    fn is_valid_cursor(&self, cursor: &Cursor) -> bool {
        if self.widget_path.path == cursor.path {
            true
        } else if cursor.path.starts_with(&self.widget_path.path) {
            self.children
                .get(cursor.path[Self::LEVEL])
                .is_some_and(|c| c.is_valid_cursor(cursor))
        } else {
            false
        }
    }

    fn handle_stage(&mut self, git: &Git, cursor: &Cursor, diff: &FileDiff) -> orfail::Result<()> {
        cursor.path.starts_with(&self.widget_path.path).or_fail()?;

        if cursor.path != self.widget_path.path {
            let i = cursor.path[Self::LEVEL];
            self.children
                .get_mut(i)
                .or_fail()?
                .handle_stage(git, cursor, diff.path(), diff.chunks().nth(i).or_fail()?)
                .or_fail()?;
        } else {
            git.stage(&diff.to_diff()).or_fail()?;
        }

        Ok(())
    }

    fn handle_discard(
        &mut self,
        git: &Git,
        cursor: &Cursor,
        diff: &FileDiff,
    ) -> orfail::Result<()> {
        cursor.path.starts_with(&self.widget_path.path).or_fail()?;

        if cursor.path != self.widget_path.path {
            let i = cursor.path[Self::LEVEL];
            self.children
                .get_mut(i)
                .or_fail()?
                .handle_discard(git, cursor, diff.path(), diff.chunks().nth(i).or_fail()?)
                .or_fail()?;
        } else {
            git.discard(&diff.to_diff()).or_fail()?;
        }

        Ok(())
    }

    fn handle_unstage(
        &mut self,
        git: &Git,
        cursor: &Cursor,
        diff: &FileDiff,
    ) -> orfail::Result<()> {
        cursor.path.starts_with(&self.widget_path.path).or_fail()?;

        if cursor.path != self.widget_path.path {
            let i = cursor.path[Self::LEVEL];
            self.children
                .get_mut(i)
                .or_fail()?
                .handle_unstage(git, cursor, diff.path(), diff.chunks().nth(i).or_fail()?)
                .or_fail()?;
        } else {
            git.unstage(&diff.to_diff()).or_fail()?;
        }

        Ok(())
    }

    pub fn rows(&self) -> usize {
        if self.expanded {
            1 + self.children.iter().map(|c| c.rows()).sum::<usize>()
        } else {
            1
        }
    }

    pub fn full_rows(&self) -> usize {
        1 + self.children.iter().map(|c| c.full_rows()).sum::<usize>()
    }

    pub fn expand_all(&mut self) {
        self.expanded = true;
        for c in &mut self.children {
            c.expand_all();
        }
    }

    pub fn current_rows(&self, cursor: &Cursor) -> Option<usize> {
        if self.widget_path.path == cursor.path {
            Some(self.rows())
        } else if cursor.path.starts_with(&self.widget_path.path) {
            let i = cursor.path[Self::LEVEL];
            self.children[i].current_rows(cursor)
        } else {
            None
        }
    }

    pub fn cursor_abs_row(&self, cursor: &Cursor) -> usize {
        match cursor.path[..Self::LEVEL].cmp(&self.widget_path.path) {
            std::cmp::Ordering::Less => 0,
            std::cmp::Ordering::Equal => {
                if cursor.path.len() == Self::LEVEL {
                    0
                } else {
                    1 + self
                        .children
                        .iter()
                        .map(|c| c.cursor_abs_row(cursor))
                        .sum::<usize>()
                }
            }
            std::cmp::Ordering::Greater => self.rows(),
        }
    }

    pub fn render(
        &self,
        canvas: &mut Canvas,
        diff: &FileDiff,
        cursor: &Cursor,
    ) -> orfail::Result<()> {
        //
        // TODO: rename handling
        let text = match diff {
            FileDiff::Update { .. } => {
                format!(
                    "{}{} modified {} ({} chunks){}",
                    match cursor.path.len() {
                        1 if self.widget_path.path.starts_with(&cursor.path[..1]) => " :",
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
                )
            }
            FileDiff::New { .. } | FileDiff::Added { .. } => {
                format!(
                    "{}{} added {}",
                    match cursor.path.len() {
                        1 if self.widget_path.path.starts_with(&cursor.path[..1]) => " :",
                        _ => "  ",
                    },
                    if self.widget_path.path == cursor.path {
                        ">|"
                    } else if cursor.path.len() == 2 {
                        " |"
                    } else {
                        "  "
                    },
                    diff.path().display()
                )
            }
            _ => {
                return Err(orfail::Failure::new("TODO"));
            }
        };
        canvas.draw_text(Text::new(&text).or_fail()?);
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
            // TODO: !self.children.is_empty()
            true
        } else if cursor.path.starts_with(&self.widget_path.path) {
            self.children.iter().any(|w| w.can_stage(cursor))
        } else {
            false
        }
    }

    pub fn can_unstage(&self, cursor: &Cursor) -> bool {
        if self.widget_path.path == cursor.path {
            // TDOO: !self.children.is_empty()
            true
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

#[derive(Debug, Clone)]
pub struct ChunkDiffWidget {
    pub old_range: Range<usize>,
    pub new_range: Range<usize>,
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
            old_range: Range {
                start: diff.old_start_line_number,
                end: diff.old_start_line_number + diff.old_columns(),
            },
            new_range: Range {
                start: diff.new_start_line_number,
                end: diff.new_start_line_number + diff.new_columns(),
            },
            widget_path,
            children,
            expanded: true,
        }
    }

    fn intersect(&self, other: &Self) -> bool {
        self.old_range.contains(&other.new_range.start)
            || self.old_range.contains(&other.new_range.end)
            || self.new_range.contains(&other.old_range.start)
            || self.new_range.contains(&other.old_range.end)
    }

    fn restore_state(&mut self, old: &[&Self]) {
        if old.is_empty() {
            return;
        }

        self.expanded = old.iter().any(|w| w.expanded);
    }

    fn get_children_len(&self, cursor: &Cursor) -> usize {
        if self.widget_path.path == cursor.path {
            self.children.len()
        } else if cursor.path.starts_with(&self.widget_path.path) {
            0
        } else {
            // TODO: error?
            0
        }
    }

    fn is_valid_cursor(&self, cursor: &Cursor) -> bool {
        if self.widget_path.path == cursor.path {
            true
        } else if cursor.path.starts_with(&self.widget_path.path) {
            self.children
                .get(cursor.path[Self::LEVEL])
                .is_some_and(|c| c.widget_path.path == cursor.path)
        } else {
            false
        }
    }

    fn handle_stage(
        &mut self,
        git: &Git,
        cursor: &Cursor,
        path: &PathBuf,
        diff: &ChunkDiff,
    ) -> orfail::Result<()> {
        cursor.path.starts_with(&self.widget_path.path).or_fail()?;

        if cursor.path != self.widget_path.path {
            let i = cursor.path[Self::LEVEL];
            self.children
                .get_mut(i)
                .or_fail()?
                .handle_stage(git, cursor, path, &diff.get_line_chunk(i, true).or_fail()?)
                .or_fail()?;
        } else {
            git.stage(&diff.to_diff(path)).or_fail()?;
        }

        Ok(())
    }

    fn handle_discard(
        &mut self,
        git: &Git,
        cursor: &Cursor,
        path: &PathBuf,
        diff: &ChunkDiff,
    ) -> orfail::Result<()> {
        cursor.path.starts_with(&self.widget_path.path).or_fail()?;

        if cursor.path != self.widget_path.path {
            let i = cursor.path[Self::LEVEL];
            self.children
                .get_mut(i)
                .or_fail()?
                .handle_discard(git, cursor, path, &diff.get_line_chunk(i, true).or_fail()?)
                .or_fail()?;
        } else {
            git.discard(&diff.to_diff(path)).or_fail()?;
        }

        Ok(())
    }

    fn handle_unstage(
        &mut self,
        git: &Git,
        cursor: &Cursor,
        path: &PathBuf,
        diff: &ChunkDiff,
    ) -> orfail::Result<()> {
        cursor.path.starts_with(&self.widget_path.path).or_fail()?;

        if cursor.path != self.widget_path.path {
            let i = cursor.path[Self::LEVEL];
            self.children
                .get_mut(i)
                .or_fail()?
                .handle_unstage(git, cursor, path, &diff.get_line_chunk(i, false).or_fail()?)
                .or_fail()?;
        } else {
            git.unstage(&diff.to_diff(path)).or_fail()?;
        }

        Ok(())
    }

    pub fn rows(&self) -> usize {
        if self.expanded {
            1 + self.children.len()
        } else {
            1
        }
    }

    pub fn full_rows(&self) -> usize {
        1 + self.children.len()
    }

    pub fn expand_all(&mut self) {
        self.expanded = true;
    }

    pub fn current_rows(&self, cursor: &Cursor) -> Option<usize> {
        if self.widget_path.path == cursor.path {
            Some(self.rows())
        } else if cursor.path.starts_with(&self.widget_path.path) {
            Some(1)
        } else {
            None
        }
    }

    pub fn cursor_abs_row(&self, cursor: &Cursor) -> usize {
        match cursor.path[..Self::LEVEL].cmp(&self.widget_path.path) {
            std::cmp::Ordering::Less => 0,
            std::cmp::Ordering::Equal => {
                if cursor.path.len() == Self::LEVEL {
                    0
                } else {
                    1 + self
                        .children
                        .iter()
                        .map(|c| c.cursor_abs_row(cursor))
                        .sum::<usize>()
                }
            }
            std::cmp::Ordering::Greater => self.rows(),
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
                    1 if self.widget_path.path.starts_with(&cursor.path[..1]) => " :  ",
                    2 if self.widget_path.path.starts_with(&cursor.path[..2]) => "   :",
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

#[derive(Debug, Clone)]
pub struct WidgetPath {
    pub path: Vec<usize>,
}

impl WidgetPath {
    pub fn new(path: Vec<usize>) -> Self {
        Self { path }
    }

    pub fn last_index(&self) -> usize {
        // bar
        self.path[self.path.len() - 1]
    }

    pub fn join(&self, index: usize) -> Self {
        let mut path = self.path.clone();
        path.push(index);
        Self::new(path)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cursor {
    pub path: Vec<usize>,
}

impl Cursor {
    pub fn new() -> Self {
        Self { path: vec![0] }
    }

    // TODO: rename
    pub fn prev(&mut self) -> bool {
        let last = self.path.last_mut().expect("infallible");
        if let Some(x) = last.checked_sub(1) {
            *last = x;
            true
        } else if self.path.len() > 1 {
            self.path.pop();
            true
        } else {
            false
        }
    }
}

#[derive(Debug, Clone)]
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

    fn handle_stage(
        &mut self,
        git: &Git,
        cursor: &Cursor,
        path: &PathBuf,
        diff: &ChunkDiff,
    ) -> orfail::Result<()> {
        cursor.path.starts_with(&self.widget_path.path).or_fail()?;

        if cursor.path == self.widget_path.path {
            git.stage(&diff.to_diff(path)).or_fail()?;
        }

        Ok(())
    }

    fn handle_discard(
        &mut self,
        git: &Git,
        cursor: &Cursor,
        path: &PathBuf,
        diff: &ChunkDiff,
    ) -> orfail::Result<()> {
        cursor.path.starts_with(&self.widget_path.path).or_fail()?;

        if cursor.path == self.widget_path.path {
            git.discard(&diff.to_diff(path)).or_fail()?;
        }

        Ok(())
    }

    fn handle_unstage(
        &mut self,
        git: &Git,
        cursor: &Cursor,
        path: &PathBuf,
        diff: &ChunkDiff,
    ) -> orfail::Result<()> {
        cursor.path.starts_with(&self.widget_path.path).or_fail()?;

        if cursor.path == self.widget_path.path {
            git.unstage(&diff.to_diff(path)).or_fail()?;
        }

        Ok(())
    }

    pub fn render(
        &self,
        canvas: &mut Canvas,
        diff: &LineDiff,
        cursor: &Cursor,
    ) -> orfail::Result<()> {
        let prefix = Text::new(&format!(
            "{}{} ",
            match cursor.path.len() {
                1 if self.widget_path.path.starts_with(&cursor.path[..1]) => " :    ",
                2 if self.widget_path.path.starts_with(&cursor.path[..2]) => "   :  ",
                3 if self.widget_path.path.starts_with(&cursor.path[..3]) => "     :",
                _ => "      ",
            },
            if self.widget_path.path == cursor.path {
                ">|"
            } else if cursor.path.len() == 4 {
                " |"
            } else {
                "  "
            }
        ))
        .or_fail()?;
        let mut text = Text::new(&format!("{}", diff)).or_fail()?;
        match diff {
            LineDiff::Old(_) => text = text.dim(),
            LineDiff::New(_) => text = text.bold(),
            LineDiff::Both(_) => {}
        }
        canvas.draw_text(prefix);
        canvas.draw_text(text);
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

    pub fn rows(&self) -> usize {
        1
    }

    pub fn cursor_abs_row(&self, cursor: &Cursor) -> usize {
        match cursor.path[..Self::LEVEL].cmp(&self.widget_path.path) {
            std::cmp::Ordering::Less => 0,
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Greater => 1,
        }
    }
}
