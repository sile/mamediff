use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use orfail::OrFail;

use crate::{
    diff::Diff,
    git::Git,
    terminal::{Canvas, Position, Terminal, Text},
};

#[derive(Debug)]
pub struct App {
    terminal: Terminal,
    exit: bool,
    git: Git,
    unstaged_diff: Diff,
    staged_diff: Diff,
    cursor: Position,
    row_offset: usize,
}

impl App {
    pub fn new() -> orfail::Result<Self> {
        let git = Git::new();
        let unstaged_diff = git.diff().or_fail()?;
        let staged_diff = git.diff_cached().or_fail()?;

        let terminal = Terminal::new().or_fail()?;
        Ok(Self {
            terminal,
            exit: false,
            git,
            unstaged_diff,
            staged_diff,
            cursor: Position::default(),
            row_offset: 0,
        })
    }

    pub fn run(mut self) -> orfail::Result<()> {
        self.render().or_fail()?;
        while !self.exit {
            let event = self.terminal.next_event().or_fail()?;
            self.handle_event(event).or_fail()?;
        }
        Ok(())
    }

    fn render(&mut self) -> orfail::Result<()> {
        let mut canvas = Canvas::new();

        canvas.draw_text(
            Text::new(&format!("Unstaged changes ({})", self.unstaged_diff.len())).or_fail()?,
        );
        canvas.draw_newline();
        canvas.draw_newline();

        canvas.draw_text(
            Text::new(&format!("Staged changes ({})", self.staged_diff.len())).or_fail()?,
        );
        canvas.draw_newline();

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
        self.unstaged_diff = self.git.diff().or_fail()?;
        self.staged_diff = self.git.diff_cached().or_fail()?;
        self.render().or_fail()?;
        Ok(())
    }
}
