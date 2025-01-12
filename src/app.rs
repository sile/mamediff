use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use orfail::OrFail;

use crate::{
    canvas::Canvas,
    diff::Diff,
    git,
    terminal::Terminal,
    widget_diff_tree::{Cursor, DiffPhase, DiffTreeNode, PhasedDiff},
    widget_legend::LegendWidget,
};

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
