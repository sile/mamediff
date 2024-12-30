use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use orfail::OrFail;

use crate::{
    diff::Diff,
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
        todo!()
    }

    fn handle_up(&mut self) -> orfail::Result<()> {
        for widget in self.widgets.iter_mut().rev().skip_while(|w| !w.focused) {
            widget.focus_prev();
            if widget.focused {
                break;
            }
        }
        Ok(())
    }

    fn handle_down(&mut self) -> orfail::Result<()> {
        for widget in self.widgets.iter_mut().skip_while(|w| !w.focused) {
            widget.focus_next();
            if widget.focused {
                break;
            }
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
        } else if !self.staged {
            self.focused = false;
        }
    }

    pub fn focus_prev(&mut self) {
        if !self.focused {
            self.focused = true;
        } else if self.staged {
            self.focused = false;
        }
    }

    pub fn reload(&mut self, git: &Git) -> orfail::Result<()> {
        self.diff = if self.staged {
            git.diff_cached().or_fail()?
        } else {
            git.diff().or_fail()?
        };
        Ok(())
    }

    pub fn render(&self, canvas: &mut Canvas) -> orfail::Result<()> {
        canvas.draw_text(
            Text::new(&format!(
                "{} {} changes ({}){}",
                if self.focused { ">" } else { " " },
                if self.staged { "Staged" } else { "Unstaged" },
                self.diff.len(),
                if self.diff.len() == 0 { "" } else { "…" }
            ))
            .or_fail()?,
        );
        canvas.draw_newline();
        canvas.draw_newline();
        Ok(())
    }

    pub fn rows(&self) -> usize {
        1
    }
}

#[derive(Debug, Default)]
pub struct FileDiffWidget {
    pub focused: bool,
    pub expanded: bool,
    pub children: Vec<ChunkDiffWidget>,
}

#[derive(Debug, Default)]
pub struct ChunkDiffWidget {
    pub focused: bool,
    pub expanded: bool,
}
