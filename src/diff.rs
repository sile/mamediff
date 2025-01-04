use std::{
    iter::Peekable,
    path::PathBuf,
    str::{FromStr, Lines},
};

use orfail::OrFail;

use crate::git::Git;

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

#[derive(Debug, Default, Clone)]
pub struct Diff {
    pub files: Vec<FileDiff>,
}

impl Diff {
    pub fn len(&self) -> usize {
        self.files.len()
    }
}

impl FromStr for Diff {
    type Err = orfail::Failure;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut lines = s.lines().peekable();
        let mut file_diffs = Vec::new();
        while let Some(file_diff) = FileDiff::parse(&mut lines).or_fail()? {
            file_diffs.push(file_diff);
        }
        Ok(Self { files: file_diffs })
    }
}

impl std::fmt::Display for Diff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for file in &self.files {
            write!(f, "{file}")?;
        }
        Ok(())
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
    pub old_start_line_number: usize,
    pub new_start_line_number: usize,
    pub start_line: Option<String>,
    pub lines: Vec<LineDiff>,
}

impl ChunkDiff {
    pub fn get_line_chunk(&self, index: usize, stage: bool) -> Option<Self> {
        if !(index < self.lines.len()) {
            return None;
        }

        let mut lines = Vec::new();
        for (i, line) in self.lines.iter().enumerate() {
            if i == index {
                lines.push(line.clone());
                continue;
            }

            match line {
                LineDiff::Old(s) if stage => {
                    lines.push(LineDiff::Both(s.clone()));
                }
                LineDiff::New(s) if !stage => {
                    lines.push(LineDiff::Both(s.clone()));
                }
                LineDiff::Both(_) => {
                    lines.push(line.clone());
                }
                _ => {}
            }
        }

        let start = if stage {
            self.old_start_line_number
        } else {
            self.new_start_line_number
        };
        Some(Self {
            old_start_line_number: start,
            new_start_line_number: start,
            start_line: self.start_line.clone(),
            lines,
        })
    }

    pub fn to_diff(&self, path: &PathBuf) -> Diff {
        let file_diff = FileDiff::Update {
            path: path.clone(),
            old_hash: "0000000".to_owned(), // dummy
            new_hash: "0000000".to_owned(), // dummy
            old_mode: None,                 // TODO
            new_mode: Mode(0),              // dummy
            content: ContentDiff::Text {
                chunks: vec![self.clone()],
            },
        };
        Diff {
            files: vec![file_diff],
        }
    }

    pub fn head_line(&self) -> String {
        let mut s = String::new();
        s.push_str(&format!(
            "@@ -{},{} +{},{} @@",
            self.old_start_line_number,
            self.old_columns(),
            self.new_start_line_number,
            self.new_columns()
        ));
        if let Some(line) = &self.start_line {
            s.push(' ');
            s.push_str(line);
        }
        s
    }

    pub fn old_columns(&self) -> usize {
        self.lines
            .iter()
            .filter(|line| matches!(line, LineDiff::Both(_) | LineDiff::Old(_)))
            .count()
    }

    pub fn new_columns(&self) -> usize {
        self.lines
            .iter()
            .filter(|line| matches!(line, LineDiff::Both(_) | LineDiff::New(_)))
            .count()
    }

    pub fn parse(lines: &mut Peekable<Lines>) -> orfail::Result<Option<Self>> {
        let Some(line) = lines.peek() else {
            return Ok(None);
        };
        if line.starts_with("diff ") {
            return Ok(None);
        }
        let line = lines.next().expect("infallible");

        line.starts_with("@@ -").or_fail()?;

        let (range_end, start_line) = if line.ends_with(" @@") {
            (line.len() - 3, None)
        } else {
            let range_end = line.find(" @@ ").or_fail()?;
            let start_line = line[range_end + " @@ ".len()..].to_owned();
            (range_end, Some(start_line))
        };

        let line = &line["@@ -".len()..range_end];
        let mut tokens = line.splitn(2, " +");
        let old_range = LineRange::from_str(tokens.next().or_fail()?).or_fail()?;
        let new_range = LineRange::from_str(tokens.next().or_fail()?).or_fail()?;

        let mut line_diffs = Vec::new();
        while lines
            .peek()
            .and_then(|line| line.chars().next())
            .is_some_and(|c| matches!(c, ' ' | '-' | '+'))
        {
            let line = lines.next().or_fail()?;
            let diff = LineDiff::from_str(line).or_fail()?;
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

impl std::fmt::Display for ChunkDiff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "@@ -{},{} +{},{} @@",
            self.old_start_line_number,
            self.old_columns(),
            self.new_start_line_number,
            self.new_columns()
        )?;
        if let Some(start) = &self.start_line {
            write!(f, " {start}")?;
        }
        writeln!(f)?;

        for line in &self.lines {
            writeln!(f, "{line}")?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum ContentDiff {
    Text { chunks: Vec<ChunkDiff> },
    Binary {},
    Empty,
}

impl ContentDiff {
    pub fn from_file(path: &PathBuf) -> orfail::Result<Self> {
        let bytes = std::fs::read(path).or_fail()?;
        if bytes.is_empty() {
            Ok(Self::Empty)
        } else if let Ok(text) = String::from_utf8(bytes.clone()) {
            // TODO: remove above clone
            Ok(Self::Text {
                chunks: vec![ChunkDiff {
                    old_start_line_number: 0,
                    new_start_line_number: 1,
                    start_line: None,
                    lines: text.lines().map(|l| LineDiff::New(l.to_string())).collect(),
                }],
            })
        } else {
            // TODO: git diff --no-index --binary /dev/null $PATH
            Ok(Self::Binary {})
        }
    }

    pub fn chunks(&self) -> impl '_ + Iterator<Item = &ChunkDiff> {
        match self {
            ContentDiff::Text { chunks } => Some(chunks.iter()).into_iter().flatten(),
            ContentDiff::Binary { .. } | ContentDiff::Empty => None.into_iter().flatten(),
        }
    }

    pub fn parse(lines: &mut Peekable<Lines>) -> orfail::Result<Self> {
        if lines.peek().map_or(true, |line| line.starts_with("diff ")) {
            return Ok(Self::Empty);
        }

        let line = lines.next().or_fail()?;
        if line.starts_with("Binary files ") {
            return Ok(Self::Binary {
                // TODO: message: line.to_owned(),
            });
        }

        line.starts_with("--- ").or_fail()?;

        let line = lines.next().or_fail()?;
        line.starts_with("+++ ").or_fail()?;

        let mut chunks = Vec::new();
        while let Some(chunk) = ChunkDiff::parse(lines).or_fail()? {
            chunks.push(chunk);
        }

        Ok(Self::Text { chunks })
    }
}

impl std::fmt::Display for ContentDiff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContentDiff::Text { chunks } => {
                for chunk in chunks {
                    write!(f, "{chunk}")?;
                }
            }
            ContentDiff::Binary {} => {
                // TODO:                write!(f, "{message}")?;
            }
            ContentDiff::Empty => {}
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum FileDiff {
    New {
        path: PathBuf,
        hash: String,
        mode: Mode,
        content: ContentDiff,
    },
    Delete {
        path: PathBuf,
        hash: String,
        mode: Mode,
        content: ContentDiff,
    },
    Update {
        path: PathBuf,
        old_hash: String,
        new_hash: String,
        old_mode: Option<Mode>,
        new_mode: Mode,
        content: ContentDiff,
    },
    Rename {
        old_path: PathBuf,
        new_path: PathBuf,
        similarity_index: SimilarityIndexHeaderLine,
    },
    Chmod {
        path: PathBuf,
        old_mode: Mode,
        new_mode: Mode,
    },
    // TODO: merge with New
    Added {
        path: PathBuf,
        diff: String,
    },
}

impl FileDiff {
    pub fn from_added_file(git: &Git, path: &PathBuf) -> orfail::Result<Self> {
        let diff = git.diff_new_file(path).or_fail()?;
        Ok(Self::Added {
            path: path.clone(),
            diff,
        })
    }

    pub fn to_diff(&self) -> Diff {
        Diff {
            files: vec![self.clone()],
        }
    }

    pub fn path(&self) -> &PathBuf {
        match self {
            FileDiff::New { path, .. }
            | FileDiff::Delete { path, .. }
            | FileDiff::Update { path, .. }
            | FileDiff::Rename { new_path: path, .. }
            | FileDiff::Added { path, .. }
            | FileDiff::Chmod { path, .. } => path,
        }
    }

    pub fn chunks(&self) -> impl '_ + Iterator<Item = &ChunkDiff> {
        match self {
            FileDiff::Delete { content, .. } | FileDiff::Update { content, .. } => {
                Some(content.chunks()).into_iter().flatten()
            }
            FileDiff::Added { .. }
            | FileDiff::New { .. }
            | FileDiff::Rename { .. }
            | FileDiff::Chmod { .. } => None.into_iter().flatten(),
        }
    }

    pub fn parse(lines: &mut Peekable<Lines>) -> orfail::Result<Option<Self>> {
        let Some(line) = lines.next() else {
            return Ok(None);
        };

        let path = line["diff --git a/".len()..].split(' ').next().or_fail()?;
        let path = PathBuf::from(path);

        let line = lines.next().or_fail()?;
        let this = if line.starts_with(IndexHeaderLine::PREFIX) {
            let index = IndexHeaderLine::from_str(line).or_fail()?;
            Self::parse_with_index(lines, path, index, None).or_fail()?
        } else if line.starts_with(NewFileModeHeaderLine::PREFIX) {
            let new_file_mode = NewFileModeHeaderLine::from_str(line).or_fail()?;
            Self::parse_with_new_file_mode(lines, path, new_file_mode).or_fail()?
        } else if line.starts_with(DeletedFileModeHeaderLine::PREFIX) {
            let deleted_file_mode = DeletedFileModeHeaderLine::from_str(line).or_fail()?;
            Self::parse_with_deleted_file_mode(lines, path, deleted_file_mode).or_fail()?
        } else if line.starts_with(OldModeHeaderLine::PREFIX) {
            let old_mode = OldModeHeaderLine::from_str(line).or_fail()?;
            Self::parse_with_old_mode(lines, path, old_mode).or_fail()?
        } else if line.starts_with(SimilarityIndexHeaderLine::PREFIX) {
            let similarity_index = SimilarityIndexHeaderLine::from_str(line).or_fail()?;
            Self::parse_with_similarity_index(lines, path, similarity_index).or_fail()?
        } else {
            return Err(orfail::Failure::new(format!(
                "Unexpected diff header line: {line:?}"
            )));
        };
        Ok(Some(this))
    }

    fn parse_with_similarity_index(
        lines: &mut Peekable<Lines>,
        _path: PathBuf,
        similarity_index: SimilarityIndexHeaderLine,
    ) -> orfail::Result<Self> {
        let line = lines.next().or_fail()?;
        let rename_from = RenameFromHeaderLine::from_str(line).or_fail()?;

        let line = lines.next().or_fail()?;
        let rename_to = RenameToHeaderLine::from_str(line).or_fail()?;

        Ok(Self::Rename {
            old_path: rename_from.path,
            new_path: rename_to.path,
            similarity_index,
        })
    }

    fn parse_with_old_mode(
        lines: &mut Peekable<Lines>,
        path: PathBuf,
        old_mode: OldModeHeaderLine,
    ) -> orfail::Result<Self> {
        let line = lines.next().or_fail()?;
        let new_mode = NewModeHeaderLine::from_str(line).or_fail()?;

        if lines.peek().is_some_and(|line| line.starts_with("diff")) {
            return Ok(Self::Chmod {
                path,
                old_mode: old_mode.mode,
                new_mode: new_mode.mode,
            });
        }

        let line = lines.next().or_fail()?;
        let mut index = IndexHeaderLine::from_str(line).or_fail()?;
        index.mode.is_none().or_fail()?;
        index.mode = Some(new_mode.mode);

        Self::parse_with_index(lines, path, index, Some(old_mode.mode)).or_fail()
    }

    fn parse_with_new_file_mode(
        lines: &mut Peekable<Lines>,
        path: PathBuf,
        new_file_mode: NewFileModeHeaderLine,
    ) -> orfail::Result<Self> {
        let line = lines.next().or_fail()?;
        let index = IndexHeaderLine::from_str(line).or_fail()?;
        index.mode.is_none().or_fail()?;
        (index.old_hash == "0000000").or_fail()?;

        let content = ContentDiff::parse(lines).or_fail()?;
        Ok(Self::New {
            path,
            hash: index.new_hash,
            mode: new_file_mode.mode,
            content,
        })
    }

    fn parse_with_deleted_file_mode(
        lines: &mut Peekable<Lines>,
        path: PathBuf,
        deleted_file_mode: DeletedFileModeHeaderLine,
    ) -> orfail::Result<Self> {
        let line = lines.next().or_fail()?;
        let index = IndexHeaderLine::from_str(line).or_fail()?;
        index.mode.is_none().or_fail()?;
        (index.new_hash == "0000000").or_fail()?;

        let content = ContentDiff::parse(lines).or_fail()?;
        Ok(Self::Delete {
            path,
            hash: index.old_hash,
            mode: deleted_file_mode.mode,
            content,
        })
    }

    fn parse_with_index(
        lines: &mut Peekable<Lines>,
        path: PathBuf,
        index: IndexHeaderLine,
        old_mode: Option<Mode>,
    ) -> orfail::Result<Self> {
        let content = ContentDiff::parse(lines).or_fail()?;
        Ok(Self::Update {
            path,
            old_hash: index.old_hash,
            new_hash: index.new_hash,
            old_mode,
            new_mode: index.mode.or_fail()?,
            content,
        })
    }
}

// TODO: Introduce a new method for generating patch file
impl std::fmt::Display for FileDiff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileDiff::New {
                path,
                mode,
                content,
                ..
            } => {
                let path = path.display();
                writeln!(f, "diff --git a/{path} b/{path}")?;
                writeln!(f, "new file mode {mode}")?;
                // TODO: writeln!(f, "index 0000000..{hash}")?;
                if !matches!(content,ContentDiff::Empty){
                    writeln!(f, "{content}")?;
                }
            },
            FileDiff::Delete {
                // path,
                // hash,
                // mode,
                // content,
                ..
            } => todo!(),
            FileDiff::Update {
                path,
                old_hash,
                new_hash,
                old_mode,
                new_mode,
                content,
            } => {
                if old_mode.is_some(){
                    todo!();
                }

                let path = path.display();
                writeln!(f, "diff --git a/{path} b/{path}")?;
                if new_mode.0 != 0{
                    // TODO: Add comment
                    writeln!(f, "index {old_hash}..{new_hash} {new_mode}")?;
                }
                writeln!(f, "--- a/{path}")?;
                writeln!(f, "+++ b/{path}")?;
                write!(f, "{content}")?;
            }
            FileDiff::Rename {
                 old_path,
                 new_path,
                // similarity_index,
                ..
            } => {
                let old_path = old_path.display();
                let new_path = new_path.display();
                writeln!(f, "diff --git a/{old_path} b/{new_path}")?;
                writeln!(f, "rename from {old_path}")?;
                writeln!(f, "rename to {new_path}")?;
            }
            FileDiff::Chmod {
                // path,
                // old_mode,
                // new_mode,
                ..
            } => todo!(),
            FileDiff::Added { diff, ..} => {
                write!(f, "{diff}")?;
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct LineRange {
    pub start: usize,
    pub count: Option<usize>,
}

impl FromStr for LineRange {
    type Err = orfail::Failure;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut tokens = s.splitn(2, ',');
        let start = usize::from_str(tokens.next().or_fail()?).or_fail()?;
        let count = tokens.next().map(usize::from_str).transpose().or_fail()?;
        Ok(Self { start, count })
    }
}

impl std::fmt::Display for LineRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(count) = self.count {
            write!(f, "{},{}", self.start, count)
        } else {
            write!(f, "{}", self.start)
        }
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
pub struct SimilarityIndexHeaderLine {
    pub percentage: u8,
}

impl SimilarityIndexHeaderLine {
    const PREFIX: &'static str = "similarity index ";
}

impl FromStr for SimilarityIndexHeaderLine {
    type Err = orfail::Failure;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.starts_with(Self::PREFIX).or_fail()?;
        s.ends_with('%').or_fail()?;
        let s = &s[Self::PREFIX.len()..s.len() - 1];
        let percentage = s.parse::<u8>().or_fail()?;
        (percentage <= 100).or_fail()?;
        Ok(Self { percentage })
    }
}

impl std::fmt::Display for SimilarityIndexHeaderLine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}%", Self::PREFIX, self.percentage)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RenameFromHeaderLine {
    pub path: PathBuf,
}

impl RenameFromHeaderLine {
    const PREFIX: &'static str = "rename from ";
}

impl FromStr for RenameFromHeaderLine {
    type Err = orfail::Failure;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.starts_with(Self::PREFIX).or_fail()?;
        let path = PathBuf::from(&s[Self::PREFIX.len()..]);
        Ok(Self { path })
    }
}

impl std::fmt::Display for RenameFromHeaderLine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", Self::PREFIX, self.path.display())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RenameToHeaderLine {
    pub path: PathBuf,
}

impl RenameToHeaderLine {
    const PREFIX: &'static str = "rename to ";
}

impl FromStr for RenameToHeaderLine {
    type Err = orfail::Failure;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.starts_with(Self::PREFIX).or_fail()?;
        let path = PathBuf::from(&s[Self::PREFIX.len()..]);
        Ok(Self { path })
    }
}

impl std::fmt::Display for RenameToHeaderLine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", Self::PREFIX, self.path.display())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NewModeHeaderLine {
    pub mode: Mode,
}

impl NewModeHeaderLine {
    const PREFIX: &'static str = "new mode ";
}

impl FromStr for NewModeHeaderLine {
    type Err = orfail::Failure;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.starts_with(Self::PREFIX).or_fail()?;
        let s = &s[Self::PREFIX.len()..];
        let mode = Mode::from_str(s).or_fail()?;
        Ok(Self { mode })
    }
}

impl std::fmt::Display for NewModeHeaderLine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", Self::PREFIX, self.mode)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OldModeHeaderLine {
    pub mode: Mode,
}

impl OldModeHeaderLine {
    const PREFIX: &'static str = "old mode ";
}

impl FromStr for OldModeHeaderLine {
    type Err = orfail::Failure;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.starts_with(Self::PREFIX).or_fail()?;
        let s = &s[Self::PREFIX.len()..];
        let mode = Mode::from_str(s).or_fail()?;
        Ok(Self { mode })
    }
}

impl std::fmt::Display for OldModeHeaderLine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", Self::PREFIX, self.mode)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NewFileModeHeaderLine {
    pub mode: Mode,
}

impl NewFileModeHeaderLine {
    const PREFIX: &'static str = "new file mode ";
}

impl FromStr for NewFileModeHeaderLine {
    type Err = orfail::Failure;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.starts_with(Self::PREFIX).or_fail()?;
        let s = &s[Self::PREFIX.len()..];
        let mode = Mode::from_str(s).or_fail()?;
        Ok(Self { mode })
    }
}

impl std::fmt::Display for NewFileModeHeaderLine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", Self::PREFIX, self.mode)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DeletedFileModeHeaderLine {
    pub mode: Mode,
}

impl DeletedFileModeHeaderLine {
    const PREFIX: &'static str = "deleted file mode ";
}

impl FromStr for DeletedFileModeHeaderLine {
    type Err = orfail::Failure;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.starts_with(Self::PREFIX).or_fail()?;
        let s = &s[Self::PREFIX.len()..];
        let mode = Mode::from_str(s).or_fail()?;
        Ok(Self { mode })
    }
}

impl std::fmt::Display for DeletedFileModeHeaderLine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", Self::PREFIX, self.mode)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IndexHeaderLine {
    pub old_hash: String,
    pub new_hash: String,
    pub mode: Option<Mode>,
}

impl IndexHeaderLine {
    const PREFIX: &'static str = "index ";
}

impl FromStr for IndexHeaderLine {
    type Err = orfail::Failure;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.starts_with(Self::PREFIX).or_fail()?;
        let s = &s[Self::PREFIX.len()..];

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
        write!(f, "{}{}..{}", Self::PREFIX, self.old_hash, self.new_hash)?;
        if let Some(mode) = self.mode {
            write!(f, " {mode}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_header_line() -> orfail::Result<()> {
        let line = "old mode 100644";
        let v = OldModeHeaderLine::from_str(line).or_fail()?;
        assert_eq!(v.mode.0, 0o100644);
        assert_eq!(v.to_string(), line);

        let line = "new mode 100755";
        let v = NewModeHeaderLine::from_str(line).or_fail()?;
        assert_eq!(v.mode.0, 0o100755);
        assert_eq!(v.to_string(), line);

        let line = "deleted file mode 100644";
        let v = DeletedFileModeHeaderLine::from_str(line).or_fail()?;
        assert_eq!(v.mode.0, 0o100644);
        assert_eq!(v.to_string(), line);

        let line = "new file mode 100644";
        let v = NewFileModeHeaderLine::from_str(line).or_fail()?;
        assert_eq!(v.mode.0, 0o100644);
        assert_eq!(v.to_string(), line);

        let line = "rename from old_name.txt";
        let v = RenameFromHeaderLine::from_str(line).or_fail()?;
        assert_eq!(v.path, PathBuf::from("old_name.txt"));
        assert_eq!(v.to_string(), line);

        let line = "rename to new_name.txt";
        let v = RenameToHeaderLine::from_str(line).or_fail()?;
        assert_eq!(v.path, PathBuf::from("new_name.txt"));
        assert_eq!(v.to_string(), line);

        let line = "similarity index 85%";
        let v = SimilarityIndexHeaderLine::from_str(line).or_fail()?;
        assert_eq!(v.percentage, 85);
        assert_eq!(v.to_string(), line);

        let line = "index a1b2c3d..4e5f6g7 100644";
        let v = IndexHeaderLine::from_str(line).or_fail()?;
        assert_eq!(v.old_hash, "a1b2c3d");
        assert_eq!(v.new_hash, "4e5f6g7");
        assert_eq!(v.mode, Some(Mode(0o100644)));
        assert_eq!(v.to_string(), line);

        let line = "index a1b2c3d..4e5f6g7";
        let v = IndexHeaderLine::from_str(line).or_fail()?;
        assert_eq!(v.old_hash, "a1b2c3d");
        assert_eq!(v.new_hash, "4e5f6g7");
        assert_eq!(v.mode, None);
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
        assert_eq!(diff.files.len(), 1);
        assert!(matches!(diff.files[0], FileDiff::Update { .. }));
        assert_eq!(diff.to_string().trim_end(), text);

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

        let diff = Diff::from_str(text).or_fail()?;
        assert_eq!(diff.files.len(), 5);
        assert!(matches!(diff.files[0], FileDiff::Rename { .. }));
        assert!(matches!(diff.files[1], FileDiff::Chmod { .. }));
        assert!(matches!(diff.files[2], FileDiff::Delete { .. }));
        assert!(matches!(diff.files[3], FileDiff::New { .. }));
        assert!(matches!(diff.files[4], FileDiff::New { .. }));

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

        let diff = Diff::from_str(text).or_fail()?;
        assert_eq!(diff.files.len(), 1);
        assert!(matches!(diff.files[0], FileDiff::Update { .. }));

        let text = r#"diff --git a/ls b/ls
new file mode 100755
index 0000000..baec60b
Binary files /dev/null and b/ls differ"#;

        let diff = Diff::from_str(text).or_fail()?;
        assert_eq!(diff.files.len(), 1);
        assert!(matches!(diff.files[0], FileDiff::New { .. }));

        let text = r#"diff --git a/ls b/ls
index baec60b..a53cdf4 100755
Binary files a/ls and b/ls differ"#;

        let diff = Diff::from_str(text).or_fail()?;
        assert_eq!(diff.files.len(), 1);
        assert!(matches!(diff.files[0], FileDiff::Update { .. }));

        Ok(())
    }
}
