use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};
use orfail::OrFail;

#[derive(Debug)]
pub struct Terminal {}

impl Terminal {
    pub fn new() -> orfail::Result<Self> {
        crossterm::execute!(std::io::stdout(), EnterAlternateScreen).or_fail()?;
        crossterm::terminal::enable_raw_mode().or_fail()?;
        Ok(Self {})
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = crossterm::execute!(std::io::stdout(), LeaveAlternateScreen);
    }
}
