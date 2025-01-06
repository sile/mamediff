use std::{
    io::Write,
    path::PathBuf,
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
        this.call(&["rev-parse", "--is-inside-work-tree"], true)
            .ok()
            .filter(|s| s.trim() == "true")
            .map(|_| this)
    }

    pub fn stage(&self, diff: &Diff) -> orfail::Result<()> {
        let patch = diff.to_string();
        self.call_with_input(&["apply", "--cached"], &patch)
            .or_fail()?;
        Ok(())
    }

    pub fn unstage(&self, diff: &Diff) -> orfail::Result<()> {
        let patch = diff.to_string();
        self.call_with_input(&["apply", "--cached", "--reverse"], &patch)
            .or_fail()?;
        Ok(())
    }

    pub fn discard(&self, diff: &Diff) -> orfail::Result<()> {
        let patch = diff.to_string();
        self.call_with_input(&["apply", "--reverse"], &patch)
            .or_fail()?;
        Ok(())
    }

    pub fn unstaged_and_staged_diffs(&self) -> orfail::Result<(Diff, Diff)> {
        let (mut unstaged_diff, staged_diff, untracked_files) =
            std::thread::scope(|s| -> orfail::Result<_> {
                let unstaged_diff_handle = s.spawn(|| {
                    let output = self.call(&["diff"], true).or_fail()?;
                    Diff::from_str(&output).or_fail()
                });
                let staged_diff_handle = s.spawn(|| {
                    let output = self.call(&["diff", "--cached"], true).or_fail()?;
                    Diff::from_str(&output).or_fail()
                });
                let untracked_files_handle = s.spawn(|| {
                    self.call(&["ls-files", "--others", "--exclude-standard"], true)
                        .or_fail()
                        .map(|output| output.lines().map(|s| s.to_owned()).collect::<Vec<_>>())
                });

                let unstaged_diff = unstaged_diff_handle
                    .join()
                    .unwrap_or_else(|e| std::panic::resume_unwind(e))
                    .or_fail()?;
                let staged_diff = staged_diff_handle
                    .join()
                    .unwrap_or_else(|e| std::panic::resume_unwind(e))
                    .or_fail()?;
                let untracked_files = untracked_files_handle
                    .join()
                    .unwrap_or_else(|e| std::panic::resume_unwind(e))
                    .or_fail()?;

                Ok((unstaged_diff, staged_diff, untracked_files))
            })
            .or_fail()?;

        std::thread::scope(|s| -> orfail::Result<_> {
            let mut handles = Vec::new();
            for path in &untracked_files {
                handles.push(s.spawn(move || {
                    // This command exits with code 1 even upon success.
                    // Therefore, specify `check_status=false` here.
                    let diff = self
                        .call(
                            &["diff", "--no-index", "--binary", "/dev/null", path],
                            false,
                        )
                        .or_fail()?;
                    Ok(FileDiff::Added {
                        path: PathBuf::from(path),
                        diff,
                    })
                }));
            }

            let mut diffs = handles
                .into_iter()
                .map(|h| h.join().unwrap_or_else(|e| std::panic::resume_unwind(e)))
                .collect::<orfail::Result<Vec<_>>>()
                .or_fail()?;

            diffs.append(&mut unstaged_diff.files);
            unstaged_diff.files = diffs;

            Ok(())
        })
        .or_fail()?;

        Ok((unstaged_diff, staged_diff))
    }

    fn call(&self, args: &[&str], check_status: bool) -> orfail::Result<String> {
        let output = Command::new("git")
            .args(args)
            .output()
            .or_fail_with(|e| format!("Failed to execute `$ git {}`: {e}", args.join(" ")))?;

        let error = |()| {
            format!(
                "Failed to execute `$ git {}`:\n{}\n",
                args.join(" "),
                String::from_utf8_lossy(&output.stderr)
            )
        };
        (!check_status || output.status.success()).or_fail_with(error)?;
        (check_status || output.stderr.is_empty()).or_fail_with(error)?;

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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn git_new() -> orfail::Result<()> {
        let dir = tempfile::tempdir().or_fail()?;
        std::env::set_current_dir(&dir).or_fail()?;

        // `dir` is not a Git directory yet.
        assert!(Git::new().is_none());

        // Directly create a `Git` instance to bypass the check.
        let git = Git {};
        git.call(&["init"], true).or_fail()?;

        // Now, `dir` is a Git directory.
        assert!(Git::new().is_some());

        Ok(())
    }
}
