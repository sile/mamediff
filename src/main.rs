use clap::Parser;
use mamediff::git::Git;
use orfail::OrFail;

#[derive(Parser)]
#[clap(version)]
struct Args {}

fn main() -> orfail::Result<()> {
    let _args = Args::parse();
    let git = Git::new();
    git.diff().or_fail()?;

    Ok(())
}
