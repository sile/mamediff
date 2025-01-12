use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use orfail::OrFail;

use crate::{
    canvas::Canvas, terminal::Terminal, widget_diff_tree::DiffTreeWidget,
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
        let tree = DiffTreeWidget::new().or_fail()?;
        let terminal = Terminal::new().or_fail()?;
        Ok(Self {
            terminal,
            exit: false,
            row_offset: 0,
            tree,
            legend: LegendWidget::default(),
        })
    }

    pub fn run(mut self) -> orfail::Result<()> {
        self.tree.expand_if_possible(self.terminal.size());
        self.render().or_fail()?;

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
                self.tree.unstage().or_fail()?;
                self.render().or_fail()?;
            }
            KeyCode::Char('s') => {
                self.tree.stage().or_fail()?;
                self.render().or_fail()?;
            }
            KeyCode::Char('D') => {
                self.tree.discard().or_fail()?;
                self.render().or_fail()?;
            }
            KeyCode::Char('h') => {
                self.legend.toggle_hide();
                self.render().or_fail()?;
            }
            KeyCode::Char('p') if ctrl => {
                if self.tree.cursor_up().or_fail()? {
                    self.scroll_if_need().or_fail()?;
                    self.render().or_fail()?;
                }
            }
            KeyCode::Up => {
                if self.tree.cursor_up().or_fail()? {
                    self.scroll_if_need().or_fail()?;
                    self.render().or_fail()?;
                }
            }
            KeyCode::Char('n') if ctrl => {
                if self.tree.cursor_down().or_fail()? {
                    self.scroll_if_need().or_fail()?;
                    self.render().or_fail()?;
                }
            }
            KeyCode::Down => {
                if self.tree.cursor_down().or_fail()? {
                    self.scroll_if_need().or_fail()?;
                    self.render().or_fail()?;
                }
            }
            KeyCode::Char('f') if ctrl => {
                if self.tree.cursor_right().or_fail()? {
                    self.scroll_if_need().or_fail()?;
                    self.render().or_fail()?;
                }
            }
            KeyCode::Right => {
                if self.tree.cursor_right().or_fail()? {
                    self.scroll_if_need().or_fail()?;
                    self.render().or_fail()?;
                }
            }
            KeyCode::Char('b') if ctrl => {
                if self.tree.cursor_left() {
                    self.scroll_if_need().or_fail()?;
                    self.render().or_fail()?;
                }
            }
            KeyCode::Left => {
                if self.tree.cursor_left() {
                    self.scroll_if_need().or_fail()?;
                    self.render().or_fail()?;
                }
            }
            KeyCode::Char('t') | KeyCode::Tab => {
                self.tree.toggle_expansion().or_fail()?;
                self.render().or_fail()?;
            }
            _ => {}
        }
        Ok(())
    }

    fn scroll_if_need(&mut self) -> orfail::Result<()> {
        // let cursor_abs_row = self.tree.cursor_row();
        // let current_rows = self
        //     .tree
        //     .root_node
        //     .get_node(&self.tree.cursor)
        //     .ok()
        //     .map(|n| n.rows())
        //     .or_fail()?;
        // let desired_end_row = cursor_abs_row + current_rows + 1;
        // if self.row_offset + self.terminal.size().rows < desired_end_row {
        //     self.row_offset =
        //         cursor_abs_row.min(desired_end_row.saturating_sub(self.terminal.size().rows));
        // }

        Ok(())
    }
}
