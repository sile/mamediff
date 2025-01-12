use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use orfail::OrFail;

use crate::{
    canvas::Canvas,
    diff::Diff,
    git,
    terminal::Terminal,
    widget_diff_tree::{DiffTreeNode, DiffTreeWidget},
    widget_legend::LegendWidget,
};

#[derive(Debug)]
pub struct App {
    terminal: Terminal,
    exit: bool,
    row_offset: usize,
    tree: DiffTreeWidget,
    legend: LegendWidget,
}

impl App {
    pub fn new() -> orfail::Result<Self> {
        let terminal = Terminal::new().or_fail()?;
        Ok(Self {
            terminal,
            exit: false,
            row_offset: 0,
            tree: DiffTreeWidget::new(),
            legend: LegendWidget::default(),
        })
    }

    pub fn run(mut self) -> orfail::Result<()> {
        self.reload_diff().or_fail()?;
        self.tree.expand_if_possible(self.terminal.size());

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

        let mut canvas = Canvas::new(self.row_offset, self.terminal.size());
        self.tree.render(&mut canvas);
        self.legend.render(&mut canvas, &self.tree);
        self.terminal.draw_frame(canvas.into_frame()).or_fail()?;

        Ok(())
    }

    fn handle_event(&mut self, event: Event) -> orfail::Result<()> {
        match event {
            Event::FocusGained => Ok(()),
            Event::FocusLost => Ok(()),
            Event::Key(event) => self.handle_key_event(event).or_fail(),
            Event::Mouse(_) => Ok(()),
            Event::Paste(_) => Ok(()),
            Event::Resize(_, _) => {
                let cursor_row = self.tree.cursor_row();
                let rows = self.terminal.size().rows;
                self.row_offset = cursor_row.saturating_sub(rows / 2);
                self.render().or_fail()
            }
        }
    }

    fn handle_key_event(&mut self, event: KeyEvent) -> orfail::Result<()> {
        if event.kind != KeyEventKind::Press {
            return Ok(());
        }

        let ctrl = event.modifiers.contains(KeyModifiers::CONTROL);
        match event.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.exit = true;
            }
            KeyCode::Char('c') if ctrl => {
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
            KeyCode::Char('p') if ctrl => {
                self.handle_up_key().or_fail()?;
                self.render().or_fail()?;
            }
            KeyCode::Up => {
                self.handle_up_key().or_fail()?;
                self.render().or_fail()?;
            }
            KeyCode::Char('n') if ctrl => {
                self.handle_down_key().or_fail()?;
                self.render().or_fail()?;
            }
            KeyCode::Down => {
                self.handle_down_key().or_fail()?;
                self.render().or_fail()?;
            }
            KeyCode::Char('f') if ctrl => {
                self.tree.cursor_right().or_fail()?;
                self.render().or_fail()?;
            }
            KeyCode::Right => {
                self.tree.cursor_right().or_fail()?;
                self.render().or_fail()?;
            }
            KeyCode::Char('b') if ctrl => {
                self.tree.cursor_left();
                self.render().or_fail()?;
            }
            KeyCode::Left => {
                self.tree.cursor_left();
                self.render().or_fail()?;
            }
            KeyCode::Char('t') | KeyCode::Tab => {
                self.tree.toggle_expansion().or_fail()?;
                self.render().or_fail()?;
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_up_key(&mut self) -> orfail::Result<()> {
        if !self.tree.cursor_up().or_fail()? {
            return Ok(());
        }

        // TODO: factor out
        let cursor_abs_row = self.tree.cursor_row();
        let current_rows = self
            .tree
            .root_node
            .get_node(&self.tree.cursor)
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

    fn handle_down_key(&mut self) -> orfail::Result<()> {
        if !self.tree.cursor_down().or_fail()? {
            return Ok(());
        }

        // TODO: factor out
        let cursor_abs_row = self.tree.cursor_row();
        let current_rows = self
            .tree
            .root_node
            .get_node(&self.tree.cursor)
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

    fn reload_diff(&mut self) -> orfail::Result<()> {
        let old_tree = self.tree.clone(); // TODO

        let (unstaged_diff, staged_diff) = git::unstaged_and_staged_diffs().or_fail()?;
        self.reload_tree(unstaged_diff, staged_diff, &old_tree)
            .or_fail()?;

        while !self.is_valid_cursor() {
            self.handle_up_key().or_fail()?;
            if !self.is_valid_cursor() {
                self.tree.cursor_left();
            }
        }
        // TODO: expand cursor position if need

        self.render().or_fail()?;
        Ok(())
    }

    fn is_valid_cursor(&self) -> bool {
        self.tree.root_node.is_valid_cursor(&self.tree.cursor)
    }

    // TODO: refactor
    fn reload_tree(
        &mut self,
        unstaged_diff: Diff,
        staged_diff: Diff,
        old_tree: &DiffTreeWidget,
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
        if self.tree.can_stage_or_discard() {
            self.tree.root_node.children[0]
                .stage(&self.tree.cursor, &self.tree.unstaged_diff.diff)
                .or_fail()?;
            self.reload_diff().or_fail()?;
        }
        Ok(())
    }

    fn handle_discard(&mut self) -> orfail::Result<()> {
        if self.tree.can_stage_or_discard() {
            self.tree.root_node.children[0]
                .discard(&self.tree.cursor, &self.tree.unstaged_diff.diff)
                .or_fail()?;
            self.reload_diff().or_fail()?;
        }
        Ok(())
    }

    fn handle_unstage(&mut self) -> orfail::Result<()> {
        if self.tree.can_unstage() {
            self.tree.root_node.children[1]
                .unstage(&self.tree.cursor, &self.tree.staged_diff.diff)
                .or_fail()?;
        }
        Ok(())
    }
}
