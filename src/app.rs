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
            canvas.draw_newline();
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
                todo!()
            }
            KeyCode::Up => {
                //
            }
            KeyCode::Down => {
                //
            }
            _ => {}
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
}

impl DiffWidget {
    pub fn new(staged: bool) -> Self {
        Self {
            staged,
            diff: Diff::default(),
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
                " {} changes ({})",
                if self.staged { "Staged" } else { "Unstaged" },
                self.diff.len()
            ))
            .or_fail()?,
        );
        canvas.draw_newline();
        Ok(())
    }
}
