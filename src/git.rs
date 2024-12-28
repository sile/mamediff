use std::{path::PathBuf, process::Command, str::FromStr};

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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HeaderLine {
    OldMode(u32),
    NewMode(u32),
    DeleteFileMode(u32),
    NewFileMode(u32),
    CopyFrom(PathBuf),
    CopyTo(PathBuf),
    RenameFrom(PathBuf),
    RenameTo(PathBuf),
    SimilarityIndex(u8),
    DissimilarityIndex(u8),
    Index(String, String, u32),
}

impl FromStr for HeaderLine {
    type Err = orfail::Failure;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.starts_with("old mode ") {
            let mode = &s["old mode ".len()..];
            (mode.len() == 6).or_fail()?;
            let mode = u32::from_str_radix(mode, 8).or_fail()?;
            Ok(Self::OldMode(mode))
        } else if s.starts_with("new mode ") {
            let mode = &s["new mode ".len()..];
            (mode.len() == 6).or_fail()?;
            let mode = u32::from_str_radix(mode, 8).or_fail()?;
            Ok(Self::NewMode(mode))
        } else if s.starts_with("delete file mode ") {
            let mode = &s["delete file mode ".len()..];
            (mode.len() == 6).or_fail()?;
            let mode = u32::from_str_radix(mode, 8).or_fail()?;
            Ok(Self::DeleteFileMode(mode))
        } else if s.starts_with("new file mode ") {
            let mode = &s["new file mode ".len()..];
            (mode.len() == 6).or_fail()?;
            let mode = u32::from_str_radix(mode, 8).or_fail()?;
            Ok(Self::NewFileMode(mode))
        } else if s.starts_with("copy from ") {
            let path = PathBuf::from(&s["copy from ".len()..]);
            Ok(Self::CopyFrom(path))
        } else if s.starts_with("copy to ") {
            let path = PathBuf::from(&s["copy to ".len()..]);
            Ok(Self::CopyTo(path))
        } else if s.starts_with("rename from ") {
            let path = PathBuf::from(&s["rename from ".len()..]);
            Ok(Self::RenameFrom(path))
        } else if s.starts_with("rename to ") {
            let path = PathBuf::from(&s["rename to ".len()..]);
            Ok(Self::RenameTo(path))
        } else if s.starts_with("similarity index ") && s.ends_with("%") {
            let percentage = s["similarity index ".len()..s.len() - 1]
                .parse::<u8>()
                .or_fail()?;
            Ok(Self::SimilarityIndex(percentage))
        } else if s.starts_with("dissimilarity index ") && s.ends_with("%") {
            let percentage = s["dissimilarity index ".len()..s.len() - 1]
                .parse::<u8>()
                .or_fail()?;
            Ok(Self::DissimilarityIndex(percentage))
        } else if s.starts_with("index ") {
            let s = &s["index ".len()..];

            let mut tokens = s.splitn(2, ' ');
            let hashes = tokens.next().or_fail()?;
            let mode = tokens.next().or_fail()?;
            (mode.len() == 6).or_fail()?;
            let mode = u32::from_str_radix(mode, 8).or_fail()?;

            let mut tokens = hashes.splitn(2, "..");
            let before_hash = tokens.next().or_fail()?.to_owned();
            let after_hash = tokens.next().or_fail()?.to_owned();

            Ok(Self::Index(before_hash, after_hash, mode))
        } else {
            Err(orfail::Failure::new(format!(
                "Unexpected diff header line: {s}"
            )))
        }
    }
}

impl std::fmt::Display for HeaderLine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OldMode(mode) => {
                write!(f, "old mode {:06o}", mode)
            }
            Self::NewMode(mode) => {
                write!(f, "new mode {:06o}", mode)
            }
            Self::DeleteFileMode(mode) => {
                write!(f, "delete file mode {:06o}", mode)
            }
            Self::NewFileMode(mode) => {
                write!(f, "new file mode {:06o}", mode)
            }
            Self::CopyFrom(path) => {
                write!(f, "copy from {}", path.display())
            }
            Self::CopyTo(path) => {
                write!(f, "copy to {}", path.display())
            }
            Self::RenameFrom(path) => {
                write!(f, "rename from {}", path.display())
            }
            Self::RenameTo(path) => {
                write!(f, "rename to {}", path.display())
            }
            Self::SimilarityIndex(percentage) => {
                write!(f, "similarity index {}%", percentage)
            }
            Self::DissimilarityIndex(percentage) => {
                write!(f, "dissimilarity index {}%", percentage)
            }
            Self::Index(before_hash, after_hash, mode) => {
                write!(f, "index {}..{} {:06o}", before_hash, after_hash, mode)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_header_line() -> orfail::Result<()> {
        let line = "old mode 100644";
        let v = line.parse::<HeaderLine>().or_fail()?;
        assert_eq!(v, HeaderLine::OldMode(0o100644));
        assert_eq!(v.to_string(), line);

        let line = "new mode 100755";
        let v = line.parse::<HeaderLine>().or_fail()?;
        assert_eq!(v, HeaderLine::NewMode(0o100755));
        assert_eq!(v.to_string(), line);

        let line = "delete file mode 100644";
        let v = line.parse::<HeaderLine>().or_fail()?;
        assert_eq!(v, HeaderLine::DeleteFileMode(0o100644));
        assert_eq!(v.to_string(), line);

        let line = "new file mode 100644";
        let v = line.parse::<HeaderLine>().or_fail()?;
        assert_eq!(v, HeaderLine::NewFileMode(0o100644));
        assert_eq!(v.to_string(), line);

        let line = "copy from src/file.txt";
        let v = line.parse::<HeaderLine>().or_fail()?;
        assert_eq!(v, HeaderLine::CopyFrom(PathBuf::from("src/file.txt")));
        assert_eq!(v.to_string(), line);

        let line = "copy to dest/file.txt";
        let v = line.parse::<HeaderLine>().or_fail()?;
        assert_eq!(v, HeaderLine::CopyTo(PathBuf::from("dest/file.txt")));
        assert_eq!(v.to_string(), line);

        let line = "rename from old_name.txt";
        let v = line.parse::<HeaderLine>().or_fail()?;
        assert_eq!(v, HeaderLine::RenameFrom(PathBuf::from("old_name.txt")));
        assert_eq!(v.to_string(), line);

        let line = "rename to new_name.txt";
        let v = line.parse::<HeaderLine>().or_fail()?;
        assert_eq!(v, HeaderLine::RenameTo(PathBuf::from("new_name.txt")));
        assert_eq!(v.to_string(), line);

        let line = "similarity index 85%";
        let v = line.parse::<HeaderLine>().or_fail()?;
        assert_eq!(v, HeaderLine::SimilarityIndex(85));
        assert_eq!(v.to_string(), line);

        let line = "dissimilarity index 15%";
        let v = line.parse::<HeaderLine>().or_fail()?;
        assert_eq!(v, HeaderLine::DissimilarityIndex(15));
        assert_eq!(v.to_string(), line);

        let line = "index a1b2c3d..4e5f6g7 100644";
        let v = line.parse::<HeaderLine>().or_fail()?;
        assert_eq!(
            v,
            HeaderLine::Index("a1b2c3d".to_owned(), "4e5f6g7".to_owned(), 0o100644)
        );
        assert_eq!(v.to_string(), line);

        Ok(())
    }
}
