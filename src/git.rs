use std::{process::Command, str::FromStr};
use crate::diff::Diff;

    pub fn diff_cached(&self) -> orfail::Result<Diff> {
        let output = Command::new("git")
            .arg("diff")
            .arg("--cached")
            .output()
            .or_fail_with(|e| format!("Failed to execute `$ git diff --cached`: {e}"))?;
        output.status.success().or_fail_with(|()| {
            format!(
                "Failed to execute `$ git diff --cached`{}{}",
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
        })?;
        let text = String::from_utf8(output.stdout).or_fail()?;
        Diff::from_str(&text).or_fail()
    // apply, apply_cached