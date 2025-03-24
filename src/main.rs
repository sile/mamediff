use mamediff::{app::App, git};
use orfail::OrFail;

fn main() -> noargs::Result<()> {
    let mut args = noargs::args();
    args.metadata_mut().app_name = env!("CARGO_PKG_NAME");
    args.metadata_mut().app_description = env!("CARGO_PKG_DESCRIPTION");

    if noargs::VERSION_FLAG.take(&mut args).is_present() {
        println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    if noargs::HELP_FLAG.take(&mut args).is_present() {
        args.metadata_mut().help_mode = true;
    }

    if let Some(help) = args.finish()? {
        print!("{help}");
        return Ok(());
    }

    if !git::is_available() {
        eprintln!("error: no `git` command found, or not a Git directory");
        std::process::exit(1);
    };
    let app = App::new().or_fail()?;
    app.run().or_fail()?;
    Ok(())
}
