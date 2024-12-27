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

#[derive(Debug)]
pub struct FileDiff {
    path: Option<PathBuf>,
}

impl FileDiff {
    pub fn new() -> Self {
        Self { path: None }
    }
    // pub fn new(line: &str) -> orfail::Result<Self> {
    //     let path = line["diff --git a/".len()..].split(' ').next().or_fail()?;
    //     Ok(Self {
    //         path: PathBuf::from(path),
    //     })
    // }
}

// TODO: rename
#[derive(Debug)]
pub enum FileDiffPhase {
    Diff,
    Index { diff: FileDiff },
    FromPath { diff: FileDiff },
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
            diff: FileDiff::new(),
        }
    }

    fn parse(&mut self) -> orfail::Result<Diff> {
        for line in self.text.lines() {
            // match &mut self.phase {
            //     LinePhase::Diff => {
            //         line.starts_with("diff --git a/").or_fail()?;
            //         self.phase = LinePhase::Index {
            //             diff: FileDiff::new(line).or_fail()?,
            //         };
            //     }
            //     LinePhase::Index { diff } => {
            //         diff.parse_index_line(line).or_fail()?;
            //         self.phase = LinePhase::FromPath { diff };
            //     }
            //     LinePhase::FromPath { diff } => todo!(),
            // }
        }
        todo!()
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

//  #[derive(Parser)]
//  #[clap(version)]
// @@ -6,5 +8,7 @@ struct Args {}

//  fn main() -> orfail::Result<()> {
//      let _args = Args::parse();
// +    let git = Git::new();
// +    git.diff().or_fail()?;
//      Ok(())
//  }
