use std::time::Duration;

use crossterm::{
    event::Event,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen},
};
use orfail::OrFail;

#[derive(Debug)]
pub struct Terminal {}

impl Terminal {
    pub fn new() -> orfail::Result<Self> {
        crossterm::execute!(std::io::stdout(), EnterAlternateScreen).or_fail()?;
        crossterm::terminal::enable_raw_mode().or_fail()?;
        Ok(Self {})
    }

    pub fn next_event(&mut self) -> orfail::Result<Event> {
        let timeout = Duration::from_secs(1);
        while !crossterm::event::poll(timeout).or_fail()? {}
        crossterm::event::read().or_fail()
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = crossterm::execute!(std::io::stdout(), LeaveAlternateScreen);
    }
}
