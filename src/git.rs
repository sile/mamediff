use std::{
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    str::FromStr,
};

use orfail::OrFail;

use crate::diff::{ContentDiff, Diff, FileDiff, Mode};

pub fn is_available() -> bool {
    // Check if `git` is accessible and we are within a Git directory.
    call(&["rev-parse", "--is-inside-work-tree"], true)
        .ok()
        .filter(|s| s.trim() == "true")
        .is_some()
}

pub fn stage(diff: &Diff) -> orfail::Result<()> {
    let patch = diff.to_patch().or_fail()?;
    call_with_input(&["apply", "--cached"], &patch).or_fail()?;
    Ok(())
}

pub fn unstage(diff: &Diff) -> orfail::Result<()> {
    let patch = diff.to_patch().or_fail()?;
    call_with_input(&["apply", "--cached", "--reverse"], &patch).or_fail()?;
    Ok(())
}

pub fn discard(diff: &Diff) -> orfail::Result<()> {
    let patch = diff.to_patch().or_fail()?;
    call_with_input(&["apply", "--reverse"], &patch).or_fail()?;
    Ok(())
}

pub fn unstaged_and_staged_diffs() -> orfail::Result<(Diff, Diff)> {
    let (mut unstaged_diff, staged_diff, untracked_files) =
        std::thread::scope(|s| -> orfail::Result<_> {
            let unstaged_diff_handle = s.spawn(|| {
                let output = call(&["diff"], true).or_fail()?;
                Diff::from_str(&output).or_fail()
            });
            let staged_diff_handle = s.spawn(|| {
                let output = call(&["diff", "--cached"], true).or_fail()?;
                Diff::from_str(&output).or_fail()
            });
            let untracked_files_handle = s.spawn(|| {
                call(&["ls-files", "--others", "--exclude-standard"], true)
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
                let content = std::fs::read(path).or_fail()?;
                if std::str::from_utf8(&content).is_ok() {
                    let diff = new_file_diff(path, false).or_fail()?;
                    FileDiff::from_str(&diff).or_fail()
                } else {
                    Ok(FileDiff::New {
                        path: PathBuf::from(path),
                        hash: "0000000".to_string(), // dummy
                        mode: Mode(0),               // dummy
                        content: ContentDiff::Binary,
                    })
                }
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

pub fn binary_file_diff<P: AsRef<Path>>(path: P) -> orfail::Result<String> {
    let path = &path.as_ref().display().to_string();
    let diff = call(&["diff", "--binary", path], true).or_fail()?;
    if diff.is_empty() {
        call(&["diff", "--binary", "--cached", path], true).or_fail()
    } else {
        Ok(diff)
    }
}

pub fn new_file_diff<P: AsRef<Path>>(path: P, binary: bool) -> orfail::Result<String> {
    // This command exits with code 1 even upon success.
    // Therefore, specify `check_status=false` here.
    let path = &path.as_ref().display().to_string();
    if binary {
        call(
            &["diff", "--no-index", "--binary", "/dev/null", path],
            false,
        )
        .or_fail()
    } else {
        call(&["diff", "--no-index", "/dev/null", path], false).or_fail()
    }
}

fn call(args: &[&str], check_status: bool) -> orfail::Result<String> {
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

fn call_with_input(args: &[&str], input: &str) -> orfail::Result<String> {
    let mut child = Command::new("git")
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn git_new() -> orfail::Result<()> {
        let dir = tempfile::tempdir().or_fail()?;
        std::env::set_current_dir(&dir).or_fail()?;

        // `dir` is not a Git directory yet.
        assert!(!is_available());

        // Directly create a `Git` instance to bypass the check.
        call(&["init"], true).or_fail()?;

        // Now, `dir` is a Git directory.
        assert!(is_available());

        Ok(())
    }
}
