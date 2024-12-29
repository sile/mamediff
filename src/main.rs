use clap::Parser;
use mamediff::git::Git;
use orfail::OrFail;

#[derive(Parser)]
#[clap(version)]
struct Args {}

fn main() -> orfail::Result<()> {
    let _args = Args::parse();
    let git = Git::new();
    let diff = git.diff().or_fail()?;
    dbg!(diff);

    let diff = git.diff_staged().or_fail()?;
    dbg!(diff);

    Ok(())
}
