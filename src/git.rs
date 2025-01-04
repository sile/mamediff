use std::{os::unix::fs::MetadataExt, path::PathBuf, process::Command, str::FromStr};

use orfail::OrFail;

use crate::diff::{ContentDiff, Diff, FileDiff, Mode};

#[derive(Debug)]
pub struct Git {}

impl Git {
    pub fn new() -> Self {
        Self {}
    }

    pub fn stage(&self, diff: &Diff) -> orfail::Result<()> {
        // TODO: Check if the diff is still up-to-date
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

    pub fn discard(&self, diff: &Diff) -> orfail::Result<()> {
        // TODO: Check if the diff is still up-to-date
        let patch = diff.to_string();

        // TODO: use pipe
        std::fs::write(".mamediff.discard.patch", &patch).or_fail()?;

        let output = Command::new("git")
            .arg("apply")
            .arg("--reverse")
            .arg(".mamediff.discard.patch") // TODO: use pipe
            .output()
            .or_fail_with(|e| format!("Failed to execute `$ git apply --reverse`: {e}"))?;
        output.status.success().or_fail_with(|()| {
            format!(
                "Failed to execute `$ git apply --reverse`{}{}",
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
        // TODO: Check if the diff is still up-to-date
        let patch = diff.to_string();

        // TODO: use pipe
        std::fs::write(".mamediff.rev.patch", &patch).or_fail()?;

        let output = Command::new("git")
            .arg("apply")
            .arg("--cached")
            .arg("--reverse")
            .arg(".mamediff.rev.patch") // TODO: use pipe
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
        let mut diff = Diff::from_str(&text).or_fail()?;

        let output = Command::new("git")
            .arg("ls-files")
            .arg("--others")
            .arg("--exclude-standard")
            .output()
            .or_fail_with(|e| format!("Failed to execute `$ git ls-files`: {e}"))?;
        output.status.success().or_fail_with(|()| {
            format!(
                "Failed to execute `$ git ls-files`{}{}",
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
        for untracked_file in text.lines() {
            let path = PathBuf::from(untracked_file);
            let mode = Mode(path.metadata().or_fail()?.mode());
            let content = ContentDiff::from_file(&path).or_fail()?;

            // TODO: optimize
            diff.files.insert(
                0,
                FileDiff::New {
                    path,
                    hash: "".to_owned(), // dummy
                    mode,
                    content,
                },
            );
        }

        Ok(diff)
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
