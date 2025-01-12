use std::cmp::Ordering;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use orfail::OrFail;

use crate::{
    canvas::{Canvas, Token},
    diff::{ChunkDiff, Diff, FileDiff},
    git,
    terminal::Terminal,
    widget_diff_tree::{Cursor, DiffTreeNodeContent, NodePath},
    widget_legend::LegendWidget,
};

const COLLAPSED_MARK: &str = "…";

#[derive(Debug)]
pub struct App {
    terminal: Terminal,
    exit: bool,
    pub cursor: Cursor, // TODO: priv
    row_offset: usize,
    tree: DiffTree,
    legend: LegendWidget,
}

impl App {
    pub fn new() -> orfail::Result<Self> {
        let terminal = Terminal::new().or_fail()?;
        Ok(Self {
            terminal,
            exit: false,
            cursor: Cursor::root(),
            row_offset: 0,
            tree: DiffTree::new(),
            legend: LegendWidget::default(),
        })
    }

    pub fn run(mut self) -> orfail::Result<()> {
        self.reload_diff_reset().or_fail()?;

        while !self.exit {
            let event = self.terminal.next_event().or_fail()?;
            self.handle_event(event).or_fail()?;
        }
        Ok(())
    }

    fn render(&mut self) -> orfail::Result<()> {
        if self.terminal.size().is_empty() {
            return Ok(());
        }

        let cursor_abs_row = self.cursor_abs_row();
        if cursor_abs_row
            .checked_sub(self.row_offset)
            .is_none_or(|p| p >= self.terminal.size().rows)
        {
            let rows = self.terminal.size().rows;
            self.row_offset = cursor_abs_row.saturating_sub(rows / 2);
        }

        let mut canvas = Canvas::new(self.row_offset, self.terminal.size());
        for (node, diff) in self.tree.children_and_diffs() {
            if !node.render_if_need(&mut canvas, &self.cursor, diff) {
                break;
            }
        }

        self.legend.render(&mut canvas, self);

        self.terminal.render(canvas.into_frame()).or_fail()?;
        Ok(())
    }

    fn cursor_abs_row(&self) -> usize {
        self.tree.root_node.cursor_row(&self.cursor)
    }

    pub fn is_togglable(&self) -> bool {
        self.tree
            .root_node
            .get_children(&self.cursor)
            .ok()
            .is_some_and(|c| !c.is_empty())
    }

    pub fn can_stage(&self) -> bool {
        self.tree.root_node.children[0]
            .can_alter(&self.cursor, &self.tree.unstaged_diff)
            .ok()
            .is_some_and(|b| b)
    }

    pub fn can_unstage(&self) -> bool {
        self.tree.root_node.children[1]
            .can_alter(&self.cursor, &self.tree.staged_diff)
            .ok()
            .is_some_and(|b| b)
    }

    fn handle_event(&mut self, event: Event) -> orfail::Result<()> {
        match event {
            Event::FocusGained => Ok(()),
            Event::FocusLost => Ok(()),
            Event::Key(event) => self.handle_key_event(event).or_fail(),
            Event::Mouse(_) => Ok(()), // TODO: Add mouse handling
            Event::Paste(_) => Ok(()),
            Event::Resize(_, _) => {
                let cursor_abs_row = self.cursor_abs_row();
                let rows = self.terminal.size().rows;
                self.row_offset = cursor_abs_row.saturating_sub(rows / 2);
                self.render().or_fail()
            }
        }
    }

    fn handle_key_event(&mut self, event: KeyEvent) -> orfail::Result<()> {
        if event.kind != KeyEventKind::Press {
            return Ok(());
        }

        match event.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.exit = true;
            }
            KeyCode::Char('c') if event.modifiers.contains(KeyModifiers::CONTROL) => {
                self.exit = true;
            }
            KeyCode::Char('u') => {
                self.handle_unstage().or_fail()?;
            }
            KeyCode::Char('s') => {
                self.handle_stage().or_fail()?;
            }
            KeyCode::Char('D') => {
                self.handle_discard().or_fail()?;
            }
            KeyCode::Char('h') => {
                self.legend.toggle_hide();
                self.render().or_fail()?;
            }
            KeyCode::Char('p') if event.modifiers.contains(KeyModifiers::CONTROL) => {
                self.handle_up().or_fail()?;
                self.render().or_fail()?;
            }
            KeyCode::Up => {
                self.handle_up().or_fail()?;
                self.render().or_fail()?;
            }
            KeyCode::Char('n') if event.modifiers.contains(KeyModifiers::CONTROL) => {
                self.handle_down().or_fail()?;
                self.render().or_fail()?;
            }
            KeyCode::Down => {
                self.handle_down().or_fail()?;
                self.render().or_fail()?;
            }
            KeyCode::Char('f') if event.modifiers.contains(KeyModifiers::CONTROL) => {
                self.handle_right().or_fail()?;
                self.render().or_fail()?;
            }
            KeyCode::Right => {
                self.handle_right().or_fail()?;
                self.render().or_fail()?;
            }
            KeyCode::Char('b') if event.modifiers.contains(KeyModifiers::CONTROL) => {
                self.handle_left().or_fail()?;
                self.render().or_fail()?;
            }
            KeyCode::Left => {
                self.handle_left().or_fail()?;
                self.render().or_fail()?;
            }
            KeyCode::Char('t') | KeyCode::Tab => {
                self.handle_tab().or_fail()?;
                self.render().or_fail()?;
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_tab(&mut self) -> orfail::Result<()> {
        self.tree.root_node.toggle(&self.cursor).or_fail()?;
        Ok(())
    }

    fn handle_up(&mut self) -> orfail::Result<()> {
        if let Some(new_cursor) = self.tree.root_node.cursor_up(&self.cursor).or_fail()? {
            self.cursor = new_cursor;
            self.tree
                .root_node
                .get_node_mut(&self.cursor)
                .or_fail()?
                .expanded = true;
        }

        // TODO: factor out
        let cursor_abs_row = self.cursor_abs_row();
        let current_rows = self
            .tree
            .root_node
            .get_node(&self.cursor)
            .ok()
            .map(|n| n.rows())
            .or_fail()?;
        let desired_end_row = cursor_abs_row + current_rows + 1;
        if self.row_offset + self.terminal.size().rows < desired_end_row {
            self.row_offset =
                cursor_abs_row.min(desired_end_row.saturating_sub(self.terminal.size().rows));
        }

        Ok(())
    }

    fn handle_down(&mut self) -> orfail::Result<()> {
        if let Some(new_cursor) = self.tree.root_node.cursor_down(&self.cursor).or_fail()? {
            self.cursor = new_cursor;
            self.tree
                .root_node
                .get_node_mut(&self.cursor)
                .or_fail()?
                .expanded = true;
        }

        // TODO: factor out
        let cursor_abs_row = self.cursor_abs_row();
        let current_rows = self
            .tree
            .root_node
            .get_node(&self.cursor)
            .ok()
            .map(|n| n.rows())
            .or_fail()?;
        let desired_end_row = cursor_abs_row + current_rows + 1;
        if self.row_offset + self.terminal.size().rows < desired_end_row {
            self.row_offset =
                cursor_abs_row.min(desired_end_row.saturating_sub(self.terminal.size().rows));
        }

        Ok(())
    }

    fn handle_right(&mut self) -> orfail::Result<()> {
        if let Some(new_cursor) = self.tree.root_node.cursor_right(&self.cursor).or_fail()? {
            self.cursor = new_cursor;
            self.tree
                .root_node
                .get_node_mut(&self.cursor)
                .or_fail()?
                .expanded = true;
        }
        Ok(())
    }

    // TODO: remove
    fn handle_left(&mut self) -> orfail::Result<()> {
        if let Some(parent) = self.cursor.parent() {
            self.cursor = parent;
        }
        Ok(())
    }

    fn reload_diff(&mut self) -> orfail::Result<()> {
        let old_tree = self.tree.clone(); // TODO

        let (unstaged_diff, staged_diff) = git::unstaged_and_staged_diffs().or_fail()?;
        self.reload_tree(unstaged_diff, staged_diff, &old_tree)
            .or_fail()?;

        while !self.is_valid_cursor() {
            self.handle_up().or_fail()?;
            if !self.is_valid_cursor() {
                self.handle_left().or_fail()?;
            }
        }
        // TODO: expand cursor position if need

        self.render().or_fail()?;
        Ok(())
    }

    fn is_valid_cursor(&self) -> bool {
        self.tree.root_node.is_valid_cursor(&self.cursor)
    }

    pub fn can_right(&self) -> bool {
        matches!(self.tree.root_node.cursor_right(&self.cursor), Ok(Some(_)))
    }

    pub fn can_down(&self) -> bool {
        matches!(self.tree.root_node.cursor_down(&self.cursor), Ok(Some(_)))
    }

    // TODO: maybe unnecessary
    fn reload_diff_reset(&mut self) -> orfail::Result<()> {
        let old_tree = DiffTree::new();
        self.cursor = Cursor::root();
        let (unstaged_diff, staged_diff) = git::unstaged_and_staged_diffs().or_fail()?;
        self.reload_tree(unstaged_diff, staged_diff, &old_tree)
            .or_fail()?;
        self.render().or_fail()?; // TODO: optimize
        Ok(())
    }

    // TODO: refactor
    fn reload_tree(
        &mut self,
        unstaged_diff: Diff,
        staged_diff: Diff,
        old_tree: &DiffTree,
    ) -> orfail::Result<()> {
        self.tree.unstaged_diff.diff = unstaged_diff;
        self.tree.staged_diff.diff = staged_diff;
        for (node, diff) in self.tree.children_and_diffs_mut() {
            node.children.clear();
            for (i, file) in diff.diff.files.iter().enumerate() {
                let path = node.path.join(i);
                let child = DiffTreeNode::new_file_diff_node(path, file);
                node.children.push(child);
            }

            node.restore_diff_node_state(
                &diff.diff,
                &old_tree
                    .children_and_diffs()
                    .map(|x| (x.0, &x.1.diff))
                    .collect::<Vec<_>>(),
            );
        }
        Ok(())
    }

    fn handle_stage(&mut self) -> orfail::Result<()> {
        if self.can_stage() {
            self.tree.root_node.children[0]
                .stage(&self.cursor, &self.tree.unstaged_diff.diff)
                .or_fail()?;
            self.reload_diff().or_fail()?;
        }
        Ok(())
    }

    fn handle_discard(&mut self) -> orfail::Result<()> {
        if self.can_stage() {
            self.tree.root_node.children[0]
                .discard(&self.cursor, &self.tree.unstaged_diff.diff)
                .or_fail()?;
            self.reload_diff().or_fail()?;
        }
        Ok(())
    }

    fn handle_unstage(&mut self) -> orfail::Result<()> {
        if self.can_unstage() {
            self.tree.root_node.children[1]
                .unstage(&self.cursor, &self.tree.staged_diff.diff)
                .or_fail()?;
        }
        Ok(())
    }
}

// TODO: move to widget_diff.rs
#[derive(Debug, Clone)]
pub struct DiffTree {
    // TODO: priv
    pub unstaged_diff: PhasedDiff,
    pub staged_diff: PhasedDiff,
    pub root_node: DiffTreeNode,
}

impl DiffTree {
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

    pub fn new() -> Self {
        Self {
            unstaged_diff: PhasedDiff {
                phase: DiffPhase::Unstaged,
                diff: Diff::default(),
            },
            staged_diff: PhasedDiff {
                phase: DiffPhase::Staged,
                diff: Diff::default(),
            },
            root_node: DiffTreeNode::new_root_node(),
        }
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
            canvas.draw(Token::new(COLLAPSED_MARK));
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

    pub fn cursor_right(&self, cursor: &Cursor) -> orfail::Result<Option<Cursor>> {
        let mut cursor = cursor.clone();

        while cursor.path.len() >= self.path.len() {
            let child_cursor = cursor.first_child();
            if self.is_valid_cursor(&child_cursor) {
                return Ok(Some(child_cursor));
            }

            let sibling_cursor = cursor.next_sibling();
            if self.is_valid_cursor(&sibling_cursor) {
                cursor = sibling_cursor;
            } else {
                break;
            }
        }

        Ok(None)
    }

    pub fn cursor_down(&self, cursor: &Cursor) -> orfail::Result<Option<Cursor>> {
        let mut cursor = cursor.clone();

        let sibling_cursor = cursor.next_sibling();
        if self.is_valid_cursor(&sibling_cursor) {
            return Ok(Some(sibling_cursor));
        }

        cursor = cursor.parent().or_fail()?;
        while self.is_valid_cursor(&cursor) {
            let sibling_cursor = cursor.next_sibling();
            if self
                .get_node(&sibling_cursor)
                .ok()
                .is_some_and(|n| !n.children.is_empty())
            {
                return Ok(Some(sibling_cursor.first_child()));
            }
            cursor = sibling_cursor;
        }

        Ok(None)
    }

    pub fn cursor_up(&self, cursor: &Cursor) -> orfail::Result<Option<Cursor>> {
        if let Some(sibling_cursor) = cursor.prev_sibling() {
            return Ok(Some(sibling_cursor));
        }

        let mut parent_cursor = cursor.parent().or_fail()?;
        while let Some(parent_sibling_cursor) = parent_cursor.prev_sibling() {
            if let Some(sibling_index) = self
                .get_node(&parent_sibling_cursor)
                .ok()
                .and_then(|node| node.children.len().checked_sub(1))
            {
                return Ok(Some(parent_sibling_cursor.join(sibling_index)));
            }
            parent_cursor = parent_sibling_cursor;
        }

        Ok(None)
    }
}
