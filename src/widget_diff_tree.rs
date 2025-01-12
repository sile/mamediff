use crate::canvas::{Canvas, Token};

#[derive(Debug)]
pub struct DiffTreeWidget {}

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
    pub path: NodePath, // TODO: priv
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
