use std::path::PathBuf;

use mamediff::{action::ActionBindingSystem, app::App, git};
use orfail::OrFail;

fn main() -> noargs::Result<()> {
    let mut args = noargs::raw_args();
    args.metadata_mut().app_name = env!("CARGO_PKG_NAME");
    args.metadata_mut().app_description = env!("CARGO_PKG_DESCRIPTION");

    if noargs::VERSION_FLAG.take(&mut args).is_present() {
        println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    noargs::HELP_FLAG.take_help(&mut args);

    let config_path: Option<PathBuf> = noargs::opt("config")
        .short('c')
        .ty("PATH")
        .doc(concat!(
            "Path to key bindings configuration file (JSONC format)\n",
            "\n",
            "Default: https://github.com/sile/mamediff/blob/main/configs/default.jsonc"
        ))
        .example("/path/to/config.jsonc")
        .env("MAMEDIFF_CONFIG_FILE")
        .take(&mut args)
        .present_and_then(|a| a.value().parse())?;

    if let Some(help) = args.finish()? {
        print!("{help}");
        return Ok(());
    }

    if !git::is_available() {
        eprintln!("error: no `git` command found, or not a Git directory");
        std::process::exit(1);
    };

    let bindings = if let Some(path) = config_path {
        ActionBindingSystem::load_from_file(path)?
    } else {
        ActionBindingSystem::load_from_str("<DEFAULT>", include_str!("../configs/default.jsonc"))?
    };

    let app = App::new(bindings).or_fail()?;
    app.run().or_fail()?;
    Ok(())
}
