use mamediff::app::App;
use orfail::OrFail;

fn main() -> orfail::Result<()> {
    let app = App::new().or_fail()?;
    app.run().or_fail()?;
    Ok(())
}
