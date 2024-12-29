use std::{
    num::NonZeroUsize,
    path::PathBuf,
    process::Command,
    str::{FromStr, Lines},
};

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
        let text = String::from_utf8(output.stdout).or_fail()?;
        Diff::from_str(&text).or_fail()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Mode(pub u32);

impl FromStr for Mode {
    type Err = orfail::Failure;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        (s.len() == 6).or_fail()?;
        let mode = u32::from_str_radix(s, 8).or_fail()?;
        Ok(Self(mode))
    }
}

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:06o}", self.0)
    }
}

#[derive(Debug, Clone)]
pub struct Diff {
    pub file_diffs: Vec<FileDiff>,
}

impl FromStr for Diff {
    type Err = orfail::Failure;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut lines = s.lines();
        let mut file_diffs = Vec::new();
        while let Some(file_diff) = FileDiff::parse(&mut lines).or_fail()? {
            file_diffs.push(file_diff);
        }
        Ok(Self { file_diffs })
    }
}

#[derive(Debug, Clone)]
pub enum LineDiff {
    Old(String),
    New(String),
    Both(String),
}

impl FromStr for LineDiff {
    type Err = orfail::Failure;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.chars().next() {
            Some('-') => Ok(Self::Old(s[1..].to_owned())),
            Some('+') => Ok(Self::New(s[1..].to_owned())),
            Some(' ') => Ok(Self::Both(s[1..].to_owned())),
            _ => Err(orfail::Failure::new(format!("Unexpected diff line: {s}"))),
        }
    }
}

impl std::fmt::Display for LineDiff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LineDiff::Old(s) => write!(f, "-{s}"),
            LineDiff::New(s) => write!(f, "+{s}"),
            LineDiff::Both(s) => write!(f, " {s}"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChunkDiff {
    pub old_start_line_number: NonZeroUsize,
    pub new_start_line_number: NonZeroUsize,
    pub start_line: String,
    pub lines: Vec<LineDiff>,
}

impl ChunkDiff {
    pub fn parse(lines: &mut Lines) -> orfail::Result<Option<Self>> {
        let Some(line) = lines.next() else {
            return Ok(None);
        };

        line.starts_with("@@ -").or_fail()?;
        let range_end = line.find(" @@ ").or_fail()?;
        let start_line = line[range_end + " @@ ".len()..].to_owned();

        let line = &line["@@ -".len()..range_end];
        let mut tokens = line.splitn(2, " +");
        let old_range = LineRange::from_str(tokens.next().or_fail()?).or_fail()?;
        let new_range = LineRange::from_str(tokens.next().or_fail()?).or_fail()?;

        let mut old_remainings = old_range.count.get();
        let mut new_remainings = new_range.count.get();

        let mut line_diffs = Vec::new();
        while old_remainings > 0 && new_remainings > 0 {
            let line = lines.next().or_fail()?;
            let diff = LineDiff::from_str(line).or_fail()?;
            match &diff {
                LineDiff::Old(_) => old_remainings = old_remainings.checked_sub(1).or_fail()?,
                LineDiff::New(_) => new_remainings = new_remainings.checked_sub(1).or_fail()?,
                LineDiff::Both(_) => {
                    old_remainings = old_remainings.checked_sub(1).or_fail()?;
                    new_remainings = new_remainings.checked_sub(1).or_fail()?;
                }
            }
            line_diffs.push(diff);
        }

        Ok(Some(Self {
            old_start_line_number: old_range.start,
            new_start_line_number: new_range.start,
            start_line,
            lines: line_diffs,
        }))
    }
}

#[derive(Debug, Clone)]
pub enum FileDiff {
    // TODO: rename,  new, delete
    Chunks {
        path: PathBuf,
        old_hash: String,
        new_hash: String,
        old_mode: Option<Mode>,
        new_mode: Mode,
        chunks: Vec<ChunkDiff>,
    },
    NewBinaryFile {
        path: PathBuf,
        hash: String,
        mode: Mode,
    },
    UpdateBinaryFile {
        path: PathBuf,
        old_hash: String,
        new_hash: String,
        old_mode: Option<Mode>,
        new_mode: Mode,
    },
}

impl FileDiff {
    pub fn parse(lines: &mut Lines) -> orfail::Result<Option<Self>> {
        let Some(line) = lines.next() else {
            return Ok(None);
        };

        let path = line["diff --git a/".len()..].split(' ').next().or_fail()?;
        let path = PathBuf::from(path);

        let line = lines.next().or_fail()?;
        let this = if line.starts_with("index ") {
            let index = IndexHeaderLine::from_str(line).or_fail()?;
            Self::parse_with_index(lines, path, index).or_fail()?
        } else if line.starts_with("new file mode ") {
            let new_file_mode = NewFileModeHeaderLine::from_str(line).or_fail()?;
            Self::parse_with_new_file_mode(lines, path, new_file_mode).or_fail()?
        } else {
            todo!()
        };
        Ok(Some(this))
    }

    fn parse_with_new_file_mode(
        lines: &mut Lines,
        path: PathBuf,
        new_file_mode: NewFileModeHeaderLine,
    ) -> orfail::Result<Self> {
        let line = lines.next().or_fail()?;
        let index = IndexHeaderLine::from_str(line).or_fail()?;
        index.mode.is_none().or_fail()?;
        (index.old_hash == "0000000").or_fail()?;

        let line = lines.next().or_fail()?;
        if line == format!("Binary files /dev/null and b/{} differ", path.display()) {
            return Ok(Self::NewBinaryFile {
                path,
                hash: index.new_hash,
                mode: new_file_mode.mode,
            });
        }

        todo!()
    }

    fn parse_with_index(
        lines: &mut Lines,
        path: PathBuf,
        index: IndexHeaderLine,
    ) -> orfail::Result<Self> {
        let line = lines.next().or_fail()?;
        if line
            == format!(
                "Binary files a/{} and b/{} differ",
                path.display(),
                path.display()
            )
        {
            return Ok(Self::UpdateBinaryFile {
                path,
                old_hash: index.old_hash,
                new_hash: index.new_hash,
                old_mode: None,
                new_mode: index.mode.or_fail()?,
            });
        }
        line.starts_with("--- a/").or_fail()?;

        let line = lines.next().or_fail()?;
        line.starts_with("+++ b/").or_fail()?;

        let mut chunks = Vec::new();
        while let Some(chunk) = ChunkDiff::parse(lines).or_fail()? {
            chunks.push(chunk);
        }

        Ok(Self::Chunks {
            path,
            old_hash: index.old_hash,
            new_hash: index.new_hash,
            old_mode: None,
            new_mode: index.mode.or_fail()?,
            chunks,
        })
    }
}

#[derive(Debug)]
pub struct LineRange {
    pub start: NonZeroUsize,
    pub count: NonZeroUsize,
}

impl FromStr for LineRange {
    type Err = orfail::Failure;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut tokens = s.splitn(2, ',');
        let start = NonZeroUsize::from_str(tokens.next().or_fail()?).or_fail()?;
        let count = NonZeroUsize::from_str(tokens.next().or_fail()?).or_fail()?;
        Ok(Self { start, count })
    }
}

impl std::fmt::Display for LineRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{},{}", self.start, self.count)
    }
}

// TODO: rename
#[derive(Debug, Default)]
pub enum FileDiffPhase {
    #[default]
    DiffHeader,
    ExtendedHeader,
    Chunk,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NewFileModeHeaderLine {
    pub mode: Mode,
}

impl FromStr for NewFileModeHeaderLine {
    type Err = orfail::Failure;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.starts_with("new file mode ").or_fail()?;
        let s = &s["new file mode ".len()..];
        let mode = Mode::from_str(s).or_fail()?;
        Ok(Self { mode })
    }
}

impl std::fmt::Display for NewFileModeHeaderLine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "new file mode {}", self.mode)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IndexHeaderLine {
    pub old_hash: String,
    pub new_hash: String,
    pub mode: Option<Mode>,
}

impl FromStr for IndexHeaderLine {
    type Err = orfail::Failure;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.starts_with("index ").or_fail()?;
        let s = &s["index ".len()..];

        let mut tokens = s.splitn(2, ' ');
        let hashes = tokens.next().or_fail()?;
        let mode = tokens.next().map(Mode::from_str).transpose().or_fail()?;

        let mut tokens = hashes.splitn(2, "..");
        let old_hash = tokens.next().or_fail()?.to_owned();
        let new_hash = tokens.next().or_fail()?.to_owned();
        Ok(Self {
            old_hash,
            new_hash,
            mode,
        })
    }
}

impl std::fmt::Display for IndexHeaderLine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "index {}..{}", self.old_hash, self.new_hash)?;
        if let Some(mode) = self.mode {
            write!(f, " {mode}")?;
        }
        Ok(())
    }
}

// https://git-scm.com/docs/diff-format#generate_patch_text_with_p
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HeaderLine {
    OldMode(u32),
    NewMode(u32),
    DeleteFileMode(u32),
    CopyFrom(PathBuf),
    CopyTo(PathBuf),
    RenameFrom(PathBuf),
    RenameTo(PathBuf),
    SimilarityIndex(u8),
    DissimilarityIndex(u8),
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
        let v = NewFileModeHeaderLine::from_str(line).or_fail()?;
        assert_eq!(v.mode.0, 0o100644);
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
        let v = IndexHeaderLine::from_str(line).or_fail()?;
        assert_eq!(v.old_hash, "a1b2c3d");
        assert_eq!(v.new_hash, "4e5f6g7");
        assert_eq!(v.mode, Some(Mode(0o100644)));
        assert_eq!(v.to_string(), line);

        Ok(())
    }

    #[test]
    fn chunks() -> orfail::Result<()> {
        let text = r#"diff --git a/src/git.rs b/src/git.rs
index e3bdb24..dd04db5 100644
--- a/src/git.rs
+++ b/src/git.rs
@@ -91,7 +91,7 @@ impl FromStr for LineDiff {
     fn from_str(s: &str) -> Result<Self, Self::Err> {
         match s.chars().next() {
             Some('-') => Ok(Self::Old(s[1..].to_owned())),
-            Some('+') => Ok(Self::New(s[1..].to_owned())),
+            Some('++') => Ok(Self::New(s[1..].to_owned())),
             Some(' ') => Ok(Self::Both(s[1..].to_owned())),
             _ => Err(orfail::Failure::new(format!("Unexpected diff line: {s}"))),
         }"#;

        let diff = Diff::from_str(text).or_fail()?;
        assert_eq!(diff.file_diffs.len(), 1);
        assert!(matches!(diff.file_diffs[0], FileDiff::Chunks { .. }));

        let text = r#"diff --git a/Cargo.toml b/C.toml
similarity index 100%
rename from Cargo.toml
rename to C.toml
diff --git a/Cargo.lock b/Cargo.lock
old mode 100644
new mode 100755
diff --git a/README.md b/README.md
deleted file mode 100644
index 977a212..0000000
--- a/README.md
+++ /dev/null
@@ -1,2 +0,0 @@
-mamediff
-========
diff --git a/foo b/foo
new file mode 100644
index 0000000..e69de29
diff --git a/lib.rs b/lib.rs
new file mode 100644
index 0000000..c2bf1c3
--- /dev/null
+++ b/lib.rs
@@ -0,0 +1 @@
+pub mod git;"#;

        let text = r#"diff --git a/Cargo.lock b/Cargo.lock
old mode 100755
new mode 100644
index 1961029..12ecda3
--- a/Cargo.lock
+++ b/Cargo.lock
@@ -8,7 +8,6 @@ version = "0.6.18"
 source = "registry+https://github.com/rust-lang/crates.io-index"
 checksum = "8acc5369981196006228e28809f761875c0327210a891e941f4c683b3a99529b"
 dependencies = [
- "anstyle",
  "anstyle-parse",
  "anstyle-query",
  "anstyle-wincon","#;

        let text = r#"diff --git a/ls b/ls
new file mode 100755
index 0000000..baec60b
Binary files /dev/null and b/ls differ"#;

        let diff = Diff::from_str(text).or_fail()?;
        assert_eq!(diff.file_diffs.len(), 1);
        assert!(matches!(diff.file_diffs[0], FileDiff::NewBinaryFile { .. }));

        let text = r#"diff --git a/ls b/ls
index baec60b..a53cdf4 100755
Binary files a/ls and b/ls differ"#;

        let diff = Diff::from_str(text).or_fail()?;
        assert_eq!(diff.file_diffs.len(), 1);
        assert!(matches!(
            diff.file_diffs[0],
            FileDiff::UpdateBinaryFile { .. }
        ));

        Ok(())
    }
}
