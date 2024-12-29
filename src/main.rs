use clap::Parser;
use mamediff::{git::Git, terminal::Terminal};
use orfail::OrFail;

#[derive(Parser)]
#[clap(version)]
struct Args {}

fn main() -> orfail::Result<()> {
    let _args = Args::parse();
    let _terminal = Terminal::new();
    let git = Git::new();
    let _diff = git.diff().or_fail()?;

    let _diff = git.diff_cached().or_fail()?;

    Ok(())
}
