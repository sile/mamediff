use std::{path::PathBuf, process::Command};

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
        DiffParser::new(&diff_text).parse().or_fail()
    }
}

#[derive(Debug)]
pub struct Diff {}

// https://git-scm.com/docs/diff-format#generate_patch_text_with_p
#[derive(Debug, Default)]
pub struct FileDiff {
    path: PathBuf,
    phase: FileDiffPhase,
}

impl FileDiff {
    pub fn parse_line(&mut self, line: &str) -> orfail::Result<bool> {
        match self.phase {
            FileDiffPhase::DiffLine => {
                let path = line["diff --git a/".len()..].split(' ').next().or_fail()?;
                self.path = PathBuf::from(path);
                self.phase = FileDiffPhase::IndexLine;
            }
            FileDiffPhase::IndexLine => todo!(),
        }
        Ok(true)
    }
}

// diff --git a/src/main.rs b/src/main.rs
// index ee157cb..90ebfea 100644
// --- a/src/main.rs
// +++ b/src/main.rs
// @@ -1,4 +1,6 @@
//  use clap::Parser;
// +use mamediff::git::Git;
// +use orfail::OrFail;
//
//  #[derive(Parser)]
//  #[clap(version)]
// @@ -6,5 +8,7 @@ struct Args {}
//
//  fn main() -> orfail::Result<()> {
//      let _args = Args::parse();
// +    let git = Git::new();
// +    git.diff().or_fail()?;
//      Ok(())
//  }

// TODO: rename
#[derive(Debug, Default)]
pub enum FileDiffPhase {
    #[default]
    DiffLine,
    IndexLine,
}

#[derive(Debug)]
pub struct DiffParser<'a> {
    text: &'a str,
    diffs: Vec<FileDiff>,
    diff: FileDiff,
}

impl<'a> DiffParser<'a> {
    fn new(text: &'a str) -> Self {
        Self {
            text,
            diffs: Vec::new(),
            diff: FileDiff::default(),
        }
    }

    fn parse(&mut self) -> orfail::Result<Diff> {
        for line in self.text.lines() {
            if self.diff.parse_line(line).or_fail()? {
                continue;
            }
            self.diffs.push(std::mem::take(&mut self.diff));
        }
        todo!()
    }
}
