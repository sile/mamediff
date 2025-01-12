use std::cmp::Ordering;

use orfail::OrFail;

use crate::{
    canvas::{Canvas, Token, TokenStyle},
    diff::{ChunkDiff, Diff, FileDiff, LineDiff},
    git,
    terminal::TerminalSize,
};

#[derive(Debug, Clone)]
pub struct DiffTreeWidget {
    // TODO: priv
    pub unstaged_diff: PhasedDiff,
    pub staged_diff: PhasedDiff,
    pub root_node: DiffTreeNode,
    pub cursor: Cursor,
}

impl DiffTreeWidget {
    pub fn new() -> orfail::Result<Self> {
        let mut this = Self {
            unstaged_diff: PhasedDiff {
                phase: DiffPhase::Unstaged,
                diff: Diff::default(),
            },
            staged_diff: PhasedDiff {
                phase: DiffPhase::Staged,
                diff: Diff::default(),
            },
            root_node: DiffTreeNode::new_root_node(),
            cursor: Cursor::root(),
        };
        this.reload().or_fail()?;
        Ok(this)
    }

    pub fn expand_if_possible(&mut self, _terminal_size: TerminalSize) {
        // TODO:
    }

    pub fn render(&self, canvas: &mut Canvas) {
        for (node, diff) in self.children_and_diffs() {
            if !node.render_if_need(canvas, &self.cursor, diff) {
                break;
            }
        }
    }

    pub fn can_cursor_up(&self) -> bool {
        matches!(self.root_node.cursor_up(&self.cursor), Some(_))
    }

    pub fn can_cursor_down(&self) -> bool {
        matches!(self.root_node.cursor_down(&self.cursor), Some(_))
    }

    pub fn can_cursor_right(&self) -> bool {
        matches!(self.root_node.cursor_right(&self.cursor), Some(_))
    }

    pub fn can_cursor_left(&self) -> bool {
        self.cursor.parent().is_some()
    }

    pub fn can_toggle(&self) -> bool {
        self.root_node
            .get_node(&self.cursor)
            .ok()
            .is_some_and(|n| !n.children.is_empty())
    }

    pub fn can_stage_or_discard(&self) -> bool {
        self.root_node.children[0]
            .can_alter(&self.cursor, &self.unstaged_diff)
            .ok()
            .is_some_and(|b| b)
    }

    pub fn can_unstage(&self) -> bool {
        self.root_node.children[1]
            .can_alter(&self.cursor, &self.staged_diff)
            .ok()
            .is_some_and(|b| b)
    }

    fn expand_parent(&mut self) -> orfail::Result<()> {
        if let Some(parent) = self.cursor.parent() {
            self.root_node.get_node_mut(&parent).or_fail()?.expanded = true;
        }
        Ok(())
    }

    pub fn cursor_up(&mut self) -> orfail::Result<bool> {
        if let Some(new_cursor) = self.root_node.cursor_up(&self.cursor) {
            self.cursor = new_cursor;
            self.expand_parent().or_fail()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn cursor_down(&mut self) -> orfail::Result<bool> {
        if let Some(new_cursor) = self.root_node.cursor_down(&self.cursor) {
            self.cursor = new_cursor;
            self.expand_parent().or_fail()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn cursor_right(&mut self) -> orfail::Result<bool> {
        if let Some(new_cursor) = self.root_node.cursor_right(&self.cursor) {
            self.cursor = new_cursor;
            self.expand_parent().or_fail()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn cursor_left(&mut self) -> bool {
        if let Some(parent) = self.cursor.parent() {
            self.cursor = parent;
            true
        } else {
            false
        }
    }

    pub fn cursor_row(&self) -> usize {
        let root_node_offset = 1;
        self.root_node.cursor_row(&self.cursor) - root_node_offset
    }

    pub fn toggle_expansion(&mut self) -> orfail::Result<()> {
        self.root_node.toggle(&self.cursor).or_fail()
    }

    pub fn current_node_rows(&self) -> orfail::Result<usize> {
        self.root_node
            .get_node(&self.cursor)
            .ok()
            .map(|n| n.rows())
            .or_fail()
    }

    // TODO: refactor
    fn reload(&mut self) -> orfail::Result<()> {
        let old = self.clone();
        let (unstaged_diff, staged_diff) = git::unstaged_and_staged_diffs().or_fail()?;

        self.unstaged_diff.diff = unstaged_diff;
        self.staged_diff.diff = staged_diff;
        for (node, diff) in self.children_and_diffs_mut() {
            node.children.clear();
            for (i, file) in diff.diff.files.iter().enumerate() {
                let path = node.path.join(i);
                let child = DiffTreeNode::new_file_diff_node(path, file);
                node.children.push(child);
            }

            node.restore_diff_node_state(
                &diff.diff,
                &old.children_and_diffs()
                    .map(|x| (x.0, &x.1.diff))
                    .collect::<Vec<_>>(),
            );
        }

        while !self.root_node.is_valid_cursor(&self.cursor) {
            if let Some(sibling_cursor) = self.cursor.prev_sibling() {
                self.cursor = sibling_cursor;
            } else if let Some(parent_cursor) = self.cursor.parent() {
                self.cursor = parent_cursor;
            } else {
                self.cursor = Cursor::root();
                break;
            }
        }

        self.expand_parent().or_fail()?;

        Ok(())
    }

    pub fn stage(&mut self) -> orfail::Result<bool> {
        if !self.can_stage_or_discard() {
            return Ok(false);
        }
        self.root_node.children[0]
            .stage(&self.cursor, &self.unstaged_diff.diff)
            .or_fail()?;
        self.reload().or_fail()?;
        Ok(true)
    }

    pub fn discard(&mut self) -> orfail::Result<bool> {
        if !self.can_stage_or_discard() {
            return Ok(false);
        }
        self.root_node.children[0]
            .discard(&self.cursor, &self.unstaged_diff.diff)
            .or_fail()?;
        self.reload().or_fail()?;
        Ok(true)
    }

    pub fn unstage(&mut self) -> orfail::Result<bool> {
        if !self.can_unstage() {
            return Ok(false);
        }
        self.root_node.children[1]
            .unstage(&self.cursor, &self.staged_diff.diff)
            .or_fail()?;
        self.reload().or_fail()?;
        Ok(true)
    }

    pub fn children_and_diffs(&self) -> impl '_ + Iterator<Item = (&DiffTreeNode, &PhasedDiff)> {
        self.root_node
            .children
            .iter()
            .zip([&self.unstaged_diff, &self.staged_diff])
    }

    pub fn children_and_diffs_mut(
        &mut self,
    ) -> impl '_ + Iterator<Item = (&mut DiffTreeNode, &mut PhasedDiff)> {
        self.root_node
            .children
            .iter_mut()
            .zip([&mut self.unstaged_diff, &mut self.staged_diff])
    }
}

#[derive(Debug, Clone)]
pub struct DiffTreeNode {
    pub path: NodePath,
    pub expanded: bool,
    pub children: Vec<Self>,
}

impl DiffTreeNode {
    pub fn new_root_node() -> Self {
        let root_path = NodePath::root();
        Self {
            path: root_path.clone(),
            expanded: true,
            children: vec![
                Self::new_diff_node(root_path.join(0)),
                Self::new_diff_node(root_path.join(1)),
            ],
        }
    }

    pub fn new_diff_node(path: NodePath) -> Self {
        Self {
            path,
            expanded: true,
            children: Vec::new(),
        }
    }

    pub fn new_file_diff_node(path: NodePath, diff: &FileDiff) -> Self {
        let children = diff
            .chunks()
            .enumerate()
            .map(|(i, c)| DiffTreeNode::new_chunk_diff_node(path.join(i), c))
            .collect();
        Self {
            path,
            expanded: false,
            children,
        }
    }

    pub fn new_chunk_diff_node(path: NodePath, diff: &ChunkDiff) -> Self {
        let children = (0..diff.lines.len())
            .map(|i| DiffTreeNode::new_line_diff_node(path.join(i)))
            .collect();
        Self {
            path,
            expanded: true,
            children,
        }
    }

    pub fn new_line_diff_node(path: NodePath) -> Self {
        Self {
            path,
            expanded: false,
            children: Vec::new(),
        }
    }

    pub fn render<T>(&self, canvas: &mut Canvas, cursor: &Cursor, content: &T)
    where
        T: DiffTreeNodeContent,
    {
        cursor.render(canvas, &self.path);
        canvas.draw(content.head_line_token());
        if !self.expanded && !self.children.is_empty() {
            canvas.draw(Token::new("…"));
        }
        canvas.newline();

        if self.expanded {
            for child in self.children.iter().zip(content.children().iter()) {
                if !child.0.render_if_need(canvas, cursor, child.1) {
                    break;
                }
            }
        }
    }

    pub fn render_if_need<T>(&self, canvas: &mut Canvas, cursor: &Cursor, content: &T) -> bool
    where
        T: DiffTreeNodeContent,
    {
        if canvas.is_frame_exceeded() {
            return false;
        }

        let mut canvas_cursor = canvas.cursor();
        let drawn_rows = self.rows();
        if canvas
            .frame_row_range()
            .start
            .checked_sub(canvas_cursor.row)
            .is_some_and(|n| n >= drawn_rows)
        {
            canvas_cursor.row += drawn_rows;
            canvas.set_cursor(canvas_cursor);
        } else {
            self.render(canvas, cursor, content);
        }
        true
    }

    pub fn rows(&self) -> usize {
        if self.expanded {
            1 + self.children.iter().map(|c| c.rows()).sum::<usize>()
        } else {
            1
        }
    }

    pub fn full_rows(&self) -> usize {
        1 + self.children.iter().map(|c| c.full_rows()).sum::<usize>()
    }

    // TODO: delete
    pub fn expand_all(&mut self) {
        self.expanded = true;
        for child in &mut self.children {
            child.expand_all();
        }
    }

    pub fn cursor_row(&self, cursor: &Cursor) -> usize {
        match cursor.path.as_slice()[..self.path.len()].cmp(self.path.as_slice()) {
            Ordering::Less => 0,
            Ordering::Equal if cursor.path.len() == self.path.len() => 0,
            Ordering::Equal => {
                1 + self
                    .children
                    .iter()
                    .map(|c| c.cursor_row(cursor))
                    .sum::<usize>()
            }
            Ordering::Greater => self.rows(),
        }
    }

    fn check_cursor(&self, cursor: &Cursor) -> orfail::Result<()> {
        cursor.path.starts_with(&self.path).or_fail_with(|()| {
            format!(
                "invalid cursor: path={:?}, cursor={:?}",
                self.path, cursor.path
            )
        })?;
        Ok(())
    }

    pub fn can_alter<T>(&self, cursor: &Cursor, content: &T) -> orfail::Result<bool>
    where
        T: DiffTreeNodeContent,
    {
        self.check_cursor(cursor).or_fail()?;

        let level = self.path.len();
        if cursor.path.len() == level {
            Ok(content.can_alter())
        } else {
            let i = cursor.path.as_slice()[level];
            let child_node = self.children.get(i).or_fail()?;
            let child_content = content.children().get(i).or_fail()?;
            child_node.can_alter(cursor, child_content).or_fail()
        }
    }

    pub fn get_children(&self, cursor: &Cursor) -> orfail::Result<&[Self]> {
        self.get_node(cursor)
            .map(|node| &node.children[..])
            .or_fail()
    }

    pub fn is_valid_cursor(&self, cursor: &Cursor) -> bool {
        self.get_node(cursor).is_ok()
    }

    pub fn toggle(&mut self, cursor: &Cursor) -> orfail::Result<()> {
        let node = self.get_node_mut(cursor).or_fail()?;
        node.expanded = !node.expanded;
        Ok(())
    }

    // TODO: refactor
    pub fn restore_diff_node_state(&mut self, diff: &Diff, old: &[(&Self, &Diff)]) {
        if old.is_empty() {
            return;
        }

        self.expanded = old.iter().any(|w| w.0.expanded);

        for (c, d) in self.children.iter_mut().zip(diff.files.iter()) {
            let old = old
                .iter()
                .flat_map(|w| w.0.children.iter().zip(w.1.files.iter()))
                .filter(|w| w.1.is_intersect(d))
                .map(|w| w.0)
                .collect::<Vec<_>>();
            c.restore_chunk_node_state(&old);
        }
    }

    // TODO: refactor
    pub fn restore_file_node_state(&mut self, diff: &FileDiff, old: &[(&Self, &FileDiff)]) {
        if old.is_empty() {
            return;
        }

        self.expanded = old.iter().any(|w| w.0.expanded);

        for (c, d) in self.children.iter_mut().zip(diff.chunks()) {
            let old = old
                .iter()
                .flat_map(|w| w.0.children.iter().zip(w.1.chunks()))
                .filter(|w| w.1.is_intersect(d))
                .map(|w| w.0)
                .collect::<Vec<_>>();
            c.restore_chunk_node_state(&old);
        }
    }

    // TODO: refactor
    pub fn restore_chunk_node_state(&mut self, old: &[&Self]) {
        if !old.is_empty() {
            self.expanded = old.iter().any(|n| n.expanded);
        }
    }

    pub fn get_node(&self, cursor: &Cursor) -> orfail::Result<&Self> {
        if let Some((_, child)) = self.get_maybe_child(cursor).or_fail()? {
            child.get_node(cursor).or_fail()
        } else {
            Ok(self)
        }
    }

    pub fn get_node_mut(&mut self, cursor: &Cursor) -> orfail::Result<&mut Self> {
        cursor.path.starts_with(&self.path).or_fail()?;

        let level = self.path.len();
        if cursor.path.len() == level {
            Ok(self)
        } else {
            let i = cursor.path.as_slice()[level];
            let child = self.children.get_mut(i).or_fail()?;
            child.get_node_mut(cursor).or_fail()
        }
    }

    pub fn get_maybe_child(&self, cursor: &Cursor) -> orfail::Result<Option<(usize, &Self)>> {
        cursor.path.starts_with(&self.path).or_fail()?;

        let level = self.path.len();
        if cursor.path.len() == level {
            Ok(None)
        } else {
            let i = cursor.path.as_slice()[level];
            let child = self.children.get(i).or_fail()?;
            Ok(Some((i, child)))
        }
    }

    pub fn stage(&self, cursor: &Cursor, diff: &Diff) -> orfail::Result<()> {
        let diff = self.get_diff(cursor, diff, true).or_fail()?;
        git::stage(&diff).or_fail()?;
        Ok(())
    }

    pub fn discard(&self, cursor: &Cursor, diff: &Diff) -> orfail::Result<()> {
        let diff = self.get_diff(cursor, diff, true).or_fail()?;
        git::discard(&diff).or_fail()?;
        Ok(())
    }

    pub fn unstage(&self, cursor: &Cursor, diff: &Diff) -> orfail::Result<()> {
        let diff = self.get_diff(cursor, diff, false).or_fail()?;
        git::unstage(&diff).or_fail()?;
        Ok(())
    }

    pub fn get_diff(&self, cursor: &Cursor, diff: &Diff, stage: bool) -> orfail::Result<Diff> {
        let Some((i, node)) = self.get_maybe_child(cursor).or_fail()? else {
            return Ok(diff.clone());
        };
        let file = diff.files.get(i).or_fail()?;
        let path = file.path();

        let Some((i, node)) = node.get_maybe_child(cursor).or_fail()? else {
            return Ok(file.to_diff());
        };
        let chunk = file.chunks_slice().get(i).or_fail()?;

        let Some((i, _node)) = node.get_maybe_child(cursor).or_fail()? else {
            return Ok(chunk.to_diff(path));
        };

        Ok(chunk.get_line_chunk(i, stage).or_fail()?.to_diff(path))
    }

    pub fn cursor_right(&self, cursor: &Cursor) -> Option<Cursor> {
        let mut cursor = cursor.clone();

        while cursor.path.len() >= self.path.len() {
            let child_cursor = cursor.first_child();
            if self.is_valid_cursor(&child_cursor) {
                return Some(child_cursor);
            }

            let sibling_cursor = cursor.next_sibling();
            if self.is_valid_cursor(&sibling_cursor) {
                cursor = sibling_cursor;
            } else {
                break;
            }
        }

        None
    }

    pub fn cursor_down(&self, cursor: &Cursor) -> Option<Cursor> {
        let sibling_cursor = cursor.next_sibling();
        if self.is_valid_cursor(&sibling_cursor) {
            return Some(sibling_cursor);
        }

        let mut base_cursor = cursor.clone();
        loop {
            base_cursor = base_cursor.parent()?;

            let mut next_cursor = base_cursor.next_sibling();
            while next_cursor.path.len() < cursor.path.len() {
                next_cursor = next_cursor.first_child();
            }

            if self.is_valid_cursor(&next_cursor) {
                return Some(next_cursor);
            }
        }
    }

    pub fn cursor_up(&self, cursor: &Cursor) -> Option<Cursor> {
        if let Some(sibling_cursor) = cursor.prev_sibling() {
            return Some(sibling_cursor);
        }

        let mut base_cursor = cursor.clone();
        loop {
            base_cursor = base_cursor.parent()?;

            let Some(mut next_cursor) = base_cursor.prev_sibling() else {
                continue;
            };
            while next_cursor.path.len() < cursor.path.len() {
                let index = self
                    .get_node(&next_cursor)
                    .ok()
                    .map(|n| n.children.len().saturating_sub(1))
                    .unwrap_or_default();
                next_cursor = next_cursor.join(index);
            }
            if self.is_valid_cursor(&next_cursor) {
                return Some(next_cursor);
            }
        }
    }
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffPhase {
    Unstaged,
    Staged,
}

#[derive(Debug, Clone)]
pub struct PhasedDiff {
    pub phase: DiffPhase,
    pub diff: Diff,
}
