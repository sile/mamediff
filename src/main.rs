use clap::Parser;

#[derive(Parser)]
#[clap(version)]
struct Args {}

fn main() -> orfail::Result<()> {
    let args = Args::parse();
    Ok(())
}
