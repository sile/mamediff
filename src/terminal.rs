use std::{io::Write, time::Duration};

use orfail::OrFail;

use crate::{
    app::Event,
    canvas::{Frame, TokenStyle},
};

#[derive(Debug)]
pub struct Terminal {
    size: TerminalSize,
    prev: Frame,
}

impl Terminal {
    pub fn new() -> orfail::Result<Self> {
        // crossterm::execute!(
        //     std::io::stdout(),
        //     EnterAlternateScreen,
        //     crossterm::cursor::Hide,
        // )
        // .or_fail()?;
        // crossterm::terminal::enable_raw_mode().or_fail()?;

        // let (cols, rows) = crossterm::terminal::size().or_fail()?;
        // let size = TerminalSize {
        //     rows: rows as usize,
        //     cols: cols as usize,
        // };

        // Ok(Self {
        //     size,
        //     prev: Frame::new(size),
        // })
        todo!()
    }

    pub fn size(&self) -> TerminalSize {
        self.size
    }

    pub fn next_event(&mut self) -> orfail::Result<Event> {
        // let timeout = Duration::from_secs(1);
        // while !crossterm::event::poll(timeout).or_fail()? {}

        // let event = crossterm::event::read().or_fail()?;
        // if let Event::Resize(cols, rows) = event {
        //     self.size.cols = cols as usize;
        //     self.size.rows = rows as usize;
        // }

        // Ok(event)
        todo!()
    }

    pub fn draw_frame(&mut self, frame: Frame) -> orfail::Result<()> {
        // let stdout = std::io::stdout();
        // let mut writer = stdout.lock();
        // for (row, line) in frame.dirty_lines(&self.prev) {
        //     crossterm::queue!(
        //         writer,
        //         crossterm::cursor::MoveTo(0, row as u16),
        //         crossterm::terminal::Clear(crossterm::terminal::ClearType::CurrentLine)
        //     )
        //     .or_fail()?;

        //     for token in line.tokens() {
        //         let attributes = match token.style() {
        //             TokenStyle::Plain => Attributes::none(),
        //             TokenStyle::Bold => Attributes::none().with(Attribute::Bold),
        //             TokenStyle::Dim => Attributes::none().with(Attribute::Dim),
        //             TokenStyle::Underlined => Attributes::none().with(Attribute::Underlined),
        //         };
        //         let content = StyledContent::new(
        //             ContentStyle {
        //                 attributes,
        //                 ..Default::default()
        //             },
        //             token.text(),
        //         );
        //     }
        // }

        // writer.flush().or_fail()?;
        // self.prev = frame;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalSize {
    pub rows: usize,
    pub cols: usize,
}

impl TerminalSize {
    pub const EMPTY: Self = Self { cols: 0, rows: 0 };

    pub fn is_empty(self) -> bool {
        self.rows == 0 || self.cols == 0
    }
}
