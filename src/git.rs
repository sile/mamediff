use std::{process::Command, str::FromStr};

use orfail::OrFail;

use crate::diff::Diff;

#[derive(Debug)]
pub struct Git {}

impl Git {
    pub fn new() -> Self {
        Self {}
    }

    pub fn stage(&self, diff: &Diff) -> orfail::Result<()> {
        let patch = diff.to_string();

        // TODO: use pipe
        std::fs::write(".mamediff.patch", &patch).or_fail()?;

        let output = Command::new("git")
            .arg("apply")
            .arg("--cached")
            .arg(".mamediff.patch") // TODO: use pipe
            .output()
            .or_fail_with(|e| format!("Failed to execute `$ git apply --cached`: {e}"))?;
        output.status.success().or_fail_with(|()| {
            format!(
                "Failed to execute `$ git apply --cached`{}{}",
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

        Ok(())
    }

    pub fn unstage(&self, diff: &Diff) -> orfail::Result<()> {
        let patch = diff.to_string();

        // TODO: use pipe
        std::fs::write(".mamediff.patch", &patch).or_fail()?;

        let output = Command::new("git")
            .arg("apply")
            .arg("--cached")
            .arg("--reverse")
            .arg(".mamediff.patch") // TODO: use pipe
            .output()
            .or_fail_with(|e| format!("Failed to execute `$ git apply --cached`: {e}"))?;
        output.status.success().or_fail_with(|()| {
            format!(
                "Failed to execute `$ git apply --cached`{}{}",
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

        Ok(())
    }

    pub fn diff(&self) -> orfail::Result<Diff> {
        // TODO: factor out
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
        let text = String::from_utf8(output.stdout).or_fail()?;
        Diff::from_str(&text).or_fail()
    }

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
            )
        })?;
        let text = String::from_utf8(output.stdout).or_fail()?;
        Diff::from_str(&text).or_fail()
    }

    // apply, apply_cached
}
