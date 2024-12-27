use std::process::Command;

use orfail::OrFail;

#[derive(Debug)]
pub struct Git {}

impl Git {
    pub fn new() -> Self {
        Self {}
    }

    pub fn diff(&self) -> orfail::Result<Diff> {
        let output = Command::new("git")
            .arg("diff")
            .output()
            .or_fail_with(|e| format!("Failed to execute `$ git diff`: {e}"))?;
        output.status.success().or_fail_with(|()| {
            format!(
                "Failed to execute `$ git diff`{}{}",
                output
                    .status
                    .code()
                    .map(|c| format!(": exit_code={c}"))
                    .unwrap_or_default(),
                (!output.stderr.is_empty())
                    .then(|| format!(
                        "\n\nSTDERR\n------\n{}\n------",
                        String::from_utf8_lossy(&output.stderr)
                    ))
                    .unwrap_or_default()
            )
        })?;
        let diff_text = String::from_utf8(output.stdout).or_fail()?;
        Diff::new(&diff_text).or_fail()
    }
}

#[derive(Debug)]
pub struct Diff {}

impl Diff {
    fn new(text: &str) -> orfail::Result<Self> {
        println!("{text}");
        todo!()
    }
}
