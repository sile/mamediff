use std::{io::Write, time::Duration};

use crossterm::{
    event::Event,
    style::{ContentStyle, StyledContent},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen},
};
use orfail::OrFail;

#[derive(Debug)]
pub struct Text {
    text: String,
    attrs: crossterm::style::Attributes,
}

impl Text {
    pub fn new(text: &str) -> orfail::Result<Self> {
        // TODO: validate
        Ok(Self {
            text: text.to_owned(),
            attrs: crossterm::style::Attributes::default(),
        })
    }

    pub fn bold(mut self) -> Self {
        self.attrs.set(crossterm::style::Attribute::Bold);
        self
    }

    pub fn dim(mut self) -> Self {
        self.attrs.set(crossterm::style::Attribute::Dim);
        self
    }
}

#[derive(Debug, Default)]
pub struct Row {
    pub texts: Vec<Text>,
}

impl Row {
    pub fn replace(&mut self, col: usize, src: Row) {
        // TODO: consider multi byte char
        let mut n = 0;
        for text in &mut self.texts {
            if n + text.text.len() < col {
                n += text.text.len();
                continue;
            }
            text.text.truncate(col - n);
            n = col;
        }
        if col > n {
            let mut padding = String::new();
            for _ in n..col {
                padding.push(' ');
            }
            self.texts.push(Text::new(&padding).expect("infallible"));
        }
        self.texts.extend(src.texts);
    }
}

// TODO: rename
#[derive(Debug)]
pub struct Canvas {
    current_row: Row,
    rows: Vec<Row>,
}

impl Canvas {
    pub fn new() -> Self {
        Self {
            current_row: Row::default(),
            rows: Vec::new(),
        }
    }

    pub fn draw_canvas(&mut self, position: Position, canvas: Canvas) {
        for (row_i, src_row) in canvas.rows.into_iter().enumerate() {
            let row_i = row_i + position.row;
            while self.rows.len() <= row_i {
                self.rows.push(Row::default());
            }

            let dst_row = &mut self.rows[row_i];
            dst_row.replace(position.col, src_row);
        }
    }

    pub fn draw_text(&mut self, text: Text) {
        self.current_row.texts.push(text);
    }

    pub fn draw_textl(&mut self, text: Text) {
        self.draw_text(text);
        self.draw_newline();
    }

    pub fn draw_newline(&mut self) {
        let last_row = std::mem::take(&mut self.current_row);
        self.rows.push(last_row);
    }

    pub fn rows(&self) -> usize {
        self.rows.len() + 1
    }
}

#[derive(Debug)]
pub struct Terminal {
    size: Size,
}

impl Terminal {
    pub fn new() -> orfail::Result<Self> {
        crossterm::execute!(
            std::io::stdout(),
            EnterAlternateScreen,
            crossterm::cursor::MoveTo(0, 0),
            crossterm::cursor::Hide,
            crossterm::terminal::DisableLineWrap
        )
        .or_fail()?;
        crossterm::terminal::enable_raw_mode().or_fail()?;

        let size = Size::current().or_fail()?;
        Ok(Self { size })
    }

    pub fn size(&self) -> Size {
        self.size
    }

    pub fn on_resized(&mut self) -> orfail::Result<()> {
        self.size = Size::current().or_fail()?;
        Ok(())
    }

    pub fn next_event(&mut self) -> orfail::Result<Event> {
        let timeout = Duration::from_secs(1);
        while !crossterm::event::poll(timeout).or_fail()? {}
        crossterm::event::read().or_fail()
    }

    pub fn render(&mut self, mut canvas: Canvas) -> orfail::Result<()> {
        canvas.draw_newline(); // TODO

        let stdout = std::io::stdout();
        let mut writer = stdout.lock();
        crossterm::queue!(
            writer,
            crossterm::terminal::Clear(crossterm::terminal::ClearType::All)
        )
        .or_fail()?;

        for (row_i, row) in canvas.rows.into_iter().enumerate() {
            crossterm::queue!(writer, crossterm::cursor::MoveTo(0, row_i as u16)).or_fail()?;

            for text in row.texts {
                if text.attrs.is_empty() {
                    crossterm::queue!(writer, crossterm::style::Print(text.text)).or_fail()?;
                } else {
                    let content = StyledContent::new(
                        ContentStyle {
                            attributes: text.attrs,
                            ..Default::default()
                        },
                        text.text,
                    );
                    crossterm::queue!(writer, crossterm::style::PrintStyledContent(content))
                        .or_fail()?;
                }
            }
        }

        writer.flush().or_fail()?;
        Ok(())
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = crossterm::execute!(std::io::stdout(), LeaveAlternateScreen);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Size {
    pub rows: usize,
    pub cols: usize,
}

impl Size {
    pub fn current() -> orfail::Result<Self> {
        let size = crossterm::terminal::size().or_fail()?;
        Ok(Self {
            rows: size.1 as usize,
            cols: size.0 as usize,
        })
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub row: usize,
    pub col: usize,
}

impl Position {
    pub fn new(row: usize, col: usize) -> Self {
        Self { row, col }
    }
}
