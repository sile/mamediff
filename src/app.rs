use orfail::OrFail;

use crate::terminal::Terminal;

#[derive(Debug)]
pub struct App {
    terminal: Terminal,
}

impl App {
    pub fn new() -> orfail::Result<Self> {
        let terminal = Terminal::new().or_fail()?;
        Ok(Self { terminal })
    }

    pub fn run(self) -> orfail::Result<()> {
        Ok(())
    }
}
