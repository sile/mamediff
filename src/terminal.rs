use std::{io::Write, time::Duration};

use crossterm::{
    event::Event,
    style::{ContentStyle, StyledContent},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen},
};
use orfail::OrFail;

use crate::canvas::Canvas;

#[derive(Debug)]
pub struct Terminal {
    size: TerminalSize,
    prev: Canvas,
}

impl Terminal {
    pub fn new() -> orfail::Result<Self> {
        crossterm::execute!(
            std::io::stdout(),
            EnterAlternateScreen,
            crossterm::cursor::Hide,
        )
        .or_fail()?;
        crossterm::terminal::enable_raw_mode().or_fail()?;

        Ok(Self {
            size: TerminalSize::current().or_fail()?,
            prev: Canvas::new(0),
        })
    }

    pub fn size(&self) -> TerminalSize {
        self.size
    }

    pub fn next_event(&mut self) -> orfail::Result<Event> {
        let timeout = Duration::from_secs(1);
        while !crossterm::event::poll(timeout).or_fail()? {}

        let event = crossterm::event::read().or_fail()?;
        if matches!(event, Event::Resize(..)) {
            self.size = TerminalSize::current().or_fail()?;
        }

        Ok(event)
    }

    pub fn render(&mut self, mut canvas: Canvas) -> orfail::Result<()> {
        canvas.draw_newline(); // TODO

        let stdout = std::io::stdout();
        let mut writer = stdout.lock();
        for (row_i, row) in canvas.rows.iter().enumerate() {
            if self.prev.rows.get(row_i) == Some(row) {
                continue;
            }

            crossterm::queue!(
                writer,
                crossterm::cursor::MoveTo(0, row_i as u16),
                crossterm::terminal::Clear(crossterm::terminal::ClearType::CurrentLine)
            )
            .or_fail()?;

            for text in &row.texts {
                if text.attrs.is_empty() {
                    crossterm::queue!(writer, crossterm::style::Print(&text.text)).or_fail()?;
                } else {
                    let content = StyledContent::new(
                        ContentStyle {
                            attributes: text.attrs,
                            ..Default::default()
                        },
                        &text.text,
                    );
                    crossterm::queue!(writer, crossterm::style::PrintStyledContent(content))
                        .or_fail()?;
                }
            }
        }

        for row_i in canvas.rows.len()..self.prev.rows.len() {
            crossterm::queue!(
                writer,
                crossterm::cursor::MoveTo(0, row_i as u16),
                crossterm::terminal::Clear(crossterm::terminal::ClearType::CurrentLine)
            )
            .or_fail()?;
        }

        writer.flush().or_fail()?;
        self.prev = canvas;
        Ok(())
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = crossterm::execute!(
            std::io::stdout(),
            LeaveAlternateScreen,
            crossterm::cursor::Show,
        );
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalSize {
    pub rows: usize,
    pub cols: usize,
}

impl TerminalSize {
    pub fn current() -> orfail::Result<Self> {
        let (cols, rows) = crossterm::terminal::size().or_fail()?;
        Ok(Self {
            rows: rows as usize,
            cols: cols as usize,
        })
    }

    pub fn is_empty(self) -> bool {
        self.rows == 0 || self.cols == 0
    }
}
