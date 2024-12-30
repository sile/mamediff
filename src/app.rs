use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use orfail::OrFail;

use crate::{
    diff::{ChunkDiff, Diff, FileDiff},
    git::Git,
    terminal::{Canvas, Terminal, Text},
};

#[derive(Debug)]
pub struct App {
    terminal: Terminal,
    exit: bool,
    git: Git,
    widgets: Vec<DiffWidget>,
}

impl App {
    pub fn new() -> orfail::Result<Self> {
        let git = Git::new();
        let terminal = Terminal::new().or_fail()?;
        Ok(Self {
            terminal,
            exit: false,
            git,
            widgets: vec![DiffWidget::new(false), DiffWidget::new(true)],
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
            widget.render(&mut canvas).or_fail()?;
        }
        self.terminal.render(canvas).or_fail()?;
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
            KeyCode::Char('q') => {
                self.exit = true;
            }
            KeyCode::Char('h') => {
                todo!()
            }
            KeyCode::Char('r') => {
                self.reload_diff().or_fail()?;
            }
            KeyCode::Char('u') => {
                todo!()
            }
            KeyCode::Char('s') => {
                todo!()
            }
            KeyCode::Char('k') => {
                todo!()
            }
            KeyCode::Char(' ') if event.modifiers.contains(KeyModifiers::CONTROL) => {
                todo!()
            }
            KeyCode::Char('g') if event.modifiers.contains(KeyModifiers::CONTROL) => {
                todo!()
            }
            KeyCode::Tab => {
                self.handle_expand().or_fail()?;
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
            _ => {}
        }
        Ok(())
    }

    fn handle_expand(&mut self) -> orfail::Result<()> {
        let widget = self
            .widgets
            .iter_mut()
            .find(|w| w.focused)
            .expect("infallible");
        widget.toggle_expand();
        Ok(())
    }

    fn handle_up(&mut self) -> orfail::Result<()> {
        for widget in self.widgets.iter_mut().rev().skip_while(|w| !w.focused) {
            widget.focus_prev();
            if widget.focused {
                return Ok(());
            }
        }
        self.widgets[0].focus_next();
        Ok(())
    }

    fn handle_down(&mut self) -> orfail::Result<()> {
        for widget in self.widgets.iter_mut().skip_while(|w| !w.focused) {
            widget.focus_next();
            if widget.focused {
                return Ok(());
            }
        }
        self.widgets.last_mut().expect("infallible").focus_prev();
        Ok(())
    }

    fn reload_diff(&mut self) -> orfail::Result<()> {
        for widget in &mut self.widgets {
            widget.reload(&self.git).or_fail()?;
        }
        self.render().or_fail()?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct DiffWidget {
    staged: bool,
    diff: Diff,
    focused: bool,
    expanded: bool,
    children: Vec<FileDiffWidget>,
}

impl DiffWidget {
    pub fn new(staged: bool) -> Self {
        Self {
            staged,
            focused: !staged,
            expanded: false,
            diff: Diff::default(),
            children: Vec::new(),
        }
    }

    pub fn focus_next(&mut self) {
        if !self.focused {
            self.focused = true;
            return;
        }

        if self.expanded {
            if self.children.iter().all(|c| !c.focused) {
                self.children[0].focused = true;
                return;
            }

            for child in self.children.iter_mut().skip_while(|c| !c.focused) {
                child.focus_next();
                if child.focused {
                    return;
                }
            }
        }

        self.focused = false;
    }

    pub fn focus_prev(&mut self) {
        if self.expanded {
            let child_focused = self.child_focused();
            if !self.focused && !child_focused {
                self.focused = true;
                self.children.last_mut().expect("infallible").focused = true;
                return;
            }

            for child in self.children.iter_mut().rev().skip_while(|c| !c.focused) {
                child.focus_prev();
                if child.focused {
                    return;
                }
            }

            if child_focused {
                return;
            }
        }

        self.focused = !self.focused;
    }

    fn child_focused(&self) -> bool {
        self.children.iter().any(|c| c.focused)
    }

    pub fn toggle_expand(&mut self) {
        if self.diff.len() != 0 {
            self.expanded = !self.expanded;
        }
    }

    pub fn reload(&mut self, git: &Git) -> orfail::Result<()> {
        self.diff = if self.staged {
            git.diff_cached().or_fail()?
        } else {
            git.diff().or_fail()?
        };

        if self.diff.files.is_empty() {
            self.expanded = false;
        }

        // TODO: merge old state (e.g., focused, expanded)
        self.children.clear();
        for file in &self.diff.files {
            self.children.push(FileDiffWidget::new(file));
        }

        Ok(())
    }

    pub fn render(&self, canvas: &mut Canvas) -> orfail::Result<()> {
        canvas.draw_text(
            Text::new(&format!(
                "{} {} changes ({}){}",
                if self.focused && !self.child_focused() {
                    ">"
                } else {
                    " "
                },
                if self.staged { "Staged" } else { "Unstaged" },
                self.diff.len(),
                if self.diff.len() == 0 || self.expanded {
                    ""
                } else {
                    "…"
                }
            ))
            .or_fail()?,
        );
        canvas.draw_newline();

        if self.expanded {
            for (child, diff) in self.children.iter().zip(self.diff.files.iter()) {
                child.render(canvas, diff).or_fail()?;
            }
        }
        canvas.draw_newline();
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct FileDiffWidget {
    pub focused: bool,
    pub expanded: bool,
    pub children: Vec<ChunkDiffWidget>,
}

impl FileDiffWidget {
    pub fn new(diff: &FileDiff) -> Self {
        Self {
            focused: false,
            expanded: false,
            children: diff.chunks().map(ChunkDiffWidget::new).collect(),
        }
    }

    pub fn render(&self, canvas: &mut Canvas, diff: &FileDiff) -> orfail::Result<()> {
        // TODO: nename handling
        canvas.draw_text(
            Text::new(&format!(
                "  {} modified {}{}",
                if self.focused { ">" } else { " " },
                diff.path().display(),
                if self.expanded { "" } else { "…" }
            ))
            .or_fail()?,
        );
        canvas.draw_newline();
        Ok(())
    }

    pub fn focus_next(&mut self) {
        // TODO: children handling
        self.focused = !self.focused;
    }

    pub fn focus_prev(&mut self) {
        // TODO: children handling
        self.focused = !self.focused;
    }
}

#[derive(Debug, Default)]
pub struct ChunkDiffWidget {
    pub focused: bool,
    pub expanded: bool,
}

impl ChunkDiffWidget {
    pub fn new(_diff: &ChunkDiff) -> Self {
        Self {
            focused: false,
            expanded: false,
        }
    }
}
