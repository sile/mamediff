use mamediff::app::App;
use orfail::OrFail;

fn main() -> orfail::Result<()> {
    // TIDO: Add -h option
    let app = App::new().or_fail()?;
    app.run().or_fail()?;
    Ok(())
}
