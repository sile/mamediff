use crate::{
    app::PhasedDiff,
    canvas::{Canvas, Token, TokenStyle},
    diff::{ChunkDiff, FileDiff, LineDiff},
};

#[derive(Debug)]
pub struct DiffTreeWidget {}

pub trait DiffTreeNodeContent {
    type Child: DiffTreeNodeContent;

    fn head_line_token(&self) -> Token;
    fn can_alter(&self) -> bool;
    fn children(&self) -> &[Self::Child];
    fn is_intersect(&self, other: &Self) -> bool;
}

impl DiffTreeNodeContent for PhasedDiff {
    type Child = FileDiff;

    fn head_line_token(&self) -> Token {
        Token::new(format!(
            "{:?} changes ({} files)",
            self.phase,
            self.diff.files.len(),
        ))
    }

    fn can_alter(&self) -> bool {
        !self.diff.files.is_empty()
    }

    fn children(&self) -> &[Self::Child] {
        &self.diff.files
    }

    fn is_intersect(&self, _other: &Self) -> bool {
        true
    }
}

impl DiffTreeNodeContent for FileDiff {
    type Child = ChunkDiff;

    fn head_line_token(&self) -> Token {
        let text = match self {
            FileDiff::Update { .. } => {
                format!(
                    "modified {} ({} chunks, -{} +{} lines)",
                    self.path().display(),
                    self.children().len(),
                    self.removed_lines(),
                    self.added_lines(),
                )
            }
            FileDiff::New { .. } | FileDiff::Added { .. } => {
                format!("added {}", self.path().display())
            }
            FileDiff::Rename { old_path, .. } => {
                format!(
                    "renamed {} -> {}",
                    old_path.display(),
                    self.path().display()
                )
            }
            FileDiff::Delete { .. } => {
                format!("deleted {}", self.path().display())
            }
            FileDiff::Chmod {
                old_mode, new_mode, ..
            } => {
                format!(
                    "mode changed {} {} -> {}",
                    self.path().display(),
                    old_mode,
                    new_mode
                )
            }
        };
        Token::new(text)
    }

    fn can_alter(&self) -> bool {
        true
    }

    fn children(&self) -> &[Self::Child] {
        self.chunks_slice()
    }

    fn is_intersect(&self, other: &Self) -> bool {
        self.path() == other.path()
    }
}

impl DiffTreeNodeContent for ChunkDiff {
    type Child = LineDiff;

    fn head_line_token(&self) -> Token {
        Token::new(self.head_line())
    }

    fn can_alter(&self) -> bool {
        true
    }

    fn children(&self) -> &[Self::Child] {
        &self.lines
    }

    fn is_intersect(&self, other: &Self) -> bool {
        let old_range = self.old_line_range();
        let new_range = self.new_line_range();
        let other_old_range = other.old_line_range();
        let other_new_range = other.new_line_range();

        old_range.contains(&other_new_range.start)
            || old_range.contains(&other_new_range.end)
            || new_range.contains(&other_old_range.start)
            || new_range.contains(&other_old_range.end)
    }
}

impl DiffTreeNodeContent for LineDiff {
    type Child = Self;

    fn head_line_token(&self) -> Token {
        let style = match self {
            LineDiff::Old(_) => TokenStyle::Dim,
            LineDiff::New(_) => TokenStyle::Bold,
            LineDiff::Both(_) => TokenStyle::Plain,
        };
        Token::with_style(self.to_string(), style)
    }

    fn can_alter(&self) -> bool {
        !matches!(self, Self::Both(_))
    }

    fn children(&self) -> &[Self::Child] {
        &[]
    }

    fn is_intersect(&self, _other: &Self) -> bool {
        false
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodePath(Vec<usize>);

impl NodePath {
    pub fn root() -> Self {
        Self(vec![0])
    }

    pub fn join(&self, index: usize) -> Self {
        let mut child = self.clone();
        child.0.push(index);
        child
    }

    pub fn starts_with(&self, other: &Self) -> bool {
        self.0.starts_with(&other.0)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    // TODO: remove
    pub fn as_slice(&self) -> &[usize] {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cursor {
    pub path: NodePath,
}

impl Cursor {
    pub fn root() -> Self {
        Self {
            path: NodePath::root().join(0),
        }
    }

    pub fn parent(&self) -> Option<Self> {
        (self.path.len() > 2).then(|| {
            let mut path = self.path.clone();
            path.0.pop();
            Self { path }
        })
    }

    pub fn first_child(&self) -> Self {
        let path = self.path.join(0);
        Self { path }
    }

    pub fn join(&self, index: usize) -> Self {
        Self {
            path: self.path.join(index),
        }
    }

    pub fn next_sibling(&self) -> Self {
        let mut path = self.path.clone();
        *path.0.last_mut().expect("infallible") += 1;
        Self { path }
    }

    pub fn prev_sibling(&self) -> Option<Self> {
        let mut path = self.path.clone();
        if path.0.last().copied() == Some(0) {
            return None;
        }
        *path.0.last_mut().expect("infallible") -= 1;
        Some(Self { path })
    }

    pub fn render(&self, canvas: &mut Canvas, path: &NodePath) {
        let mut text = String::with_capacity(path.len() * 2);
        let selected = *path == self.path;

        if selected {
            text.push('-');
        } else {
            text.push(' ');
        }

        for i in 2..path.len() {
            if i == self.path.len() && path.starts_with(&self.path) {
                text.push_str(" :")
            } else if selected {
                text.push_str("--")
            } else {
                text.push_str("  ")
            }
        }

        if selected {
            text.push_str(">| ");
        } else if path.len() == self.path.len() {
            text.push_str(" | ");
        } else {
            text.push_str("   ");
        }

        canvas.draw(Token::new(text));
    }
}
