use clap::Parser;
use mamediff::{app::App, git::Git, terminal::Terminal};
use orfail::OrFail;

#[derive(Parser)]
#[clap(version)]
struct Args {}

fn main() -> orfail::Result<()> {
    let _args = Args::parse();
    let app = App::new().or_fail()?;
    app.run().or_fail()?;
    Ok(())
}
