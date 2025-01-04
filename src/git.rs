use std::{path::PathBuf, process::Command, str::FromStr};

use orfail::OrFail;

use crate::diff::{Diff, FileDiff};

#[derive(Debug)]
pub struct Git {}

impl Git {
    pub fn new() -> Self {
        // TODO: check git command and directory
        Self {}
    }

    fn call(&self, args: &[&str], check_status: bool) -> orfail::Result<String> {
        let output = Command::new("git")
            .args(args)
            .output()
            .or_fail_with(|e| format!("Failed to execute `$ git {}`: {e}", args.join(" ")))?;

        (!check_status || output.status.success()).or_fail_with(|()| {
            format!(
                "Failed to execute `$ git {}`:\n{}\n",
                args.join(" "),
                String::from_utf8_lossy(&output.stderr)
            )
        })?;
        (check_status || output.stderr.is_empty()).or_fail_with(|()| {
            format!(
                "Failed to execute `$ git {}`:\n{}\n",
                args.join(" "),
                String::from_utf8_lossy(&output.stderr)
            )
        })?;

        String::from_utf8(output.stdout).or_fail()
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
        let output = self.call(&["diff"], true).or_fail()?;
        let mut diff = Diff::from_str(&output).or_fail()?;

        let output = self
            .call(&["ls-files", "--others", "--exclude-standard"], true)
            .or_fail()?;
        for untracked_file in output.lines() {
            let file_diff =
                FileDiff::from_added_file(self, &PathBuf::from(untracked_file)).or_fail()?;

            // TODO: optimize
            diff.files.insert(0, file_diff);
        }

        Ok(diff)
    }

    pub fn diff_new_file(&self, path: &PathBuf) -> orfail::Result<String> {
        let path = path.to_str().or_fail()?;

        // This command exits with code 1 even upon success.
        // Therefore, specify `check_status=false` here.
        let diff = self
            .call(
                &["diff", "--no-index", "--binary", "/dev/null", path],
                false,
            )
            .or_fail()?;

        Ok(diff)
    }

    pub fn diff_cached(&self) -> orfail::Result<Diff> {
        let output = self.call(&["diff", "--cached"], true).or_fail()?;
        Diff::from_str(&output).or_fail()
    }
}
