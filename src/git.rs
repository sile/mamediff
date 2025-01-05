use std::{
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    str::FromStr,
};

use orfail::OrFail;

use crate::diff::{Diff, FileDiff};

#[derive(Debug)]
pub struct Git {}

impl Git {
    pub fn new() -> Option<Self> {
        let this = Self {};

        // Check if `git` is accessible and we are within a Git directory.
        this.call(&["status"], true).ok().map(|_| this)
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

    fn call_with_input(&self, args: &[&str], input: &str) -> orfail::Result<String> {
        let mut child = Command::new("git")
            .args(args)
            .stdin(Stdio::piped())
            .spawn()
            .or_fail_with(|e| format!("Failed to execute `$ git {}`: {e}", args.join(" ")))?;

        let mut stdin = child.stdin.take().or_fail()?;
        stdin.write_all(input.as_bytes()).or_fail()?;
        std::mem::drop(stdin);

        let output = child
            .wait_with_output()
            .or_fail_with(|e| format!("Failed to execute `$ git {}`: {e}", args.join(" ")))?;

        output.status.success().or_fail_with(|()| {
            let _ = std::fs::write(".mamediff.error.input", input.as_bytes());
            format!(
                "Failed to execute `$ cat .mamediff.error.input | git {}`:\n{}\n",
                args.join(" "),
                String::from_utf8_lossy(&output.stderr)
            )
        })?;

        String::from_utf8(output.stdout).or_fail()
    }

    pub fn stage(&self, diff: &Diff) -> orfail::Result<()> {
        // TODO: Check if the diff is still up-to-date
        let patch = diff.to_string();
        self.call_with_input(&["apply", "--cached"], &patch)
            .or_fail()?;
        Ok(())
    }

    pub fn discard(&self, diff: &Diff) -> orfail::Result<()> {
        // TODO: Check if the diff is still up-to-date
        let patch = diff.to_string();
        self.call_with_input(&["apply", "--reverse"], &patch)
            .or_fail()?;
        Ok(())
    }

    pub fn unstage(&self, diff: &Diff) -> orfail::Result<()> {
        // TODO: Check if the diff is still up-to-date
        let patch = diff.to_string();
        self.call_with_input(&["apply", "--cached", "--reverse"], &patch)
            .or_fail()?;
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

    pub fn diff_new_file(&self, path: &Path) -> orfail::Result<String> {
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
