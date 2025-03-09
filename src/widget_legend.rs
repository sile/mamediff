use crate::{
    canvas::{Canvas, Token, TokenPosition},
    widget_diff_tree::DiffTreeWidget,
};

#[derive(Debug, Default)]
pub struct LegendWidget {
    pub hide: bool,
}

impl LegendWidget {
    const SHOW_COLS: usize = 19;
    const HIDE_COLS: usize = 11;

    pub fn render(&self, canvas: &mut Canvas, tree: &DiffTreeWidget) {
        if canvas.frame_size().cols <= Self::SHOW_COLS {
            return;
        }

        canvas.set_cursor(TokenPosition::row(canvas.frame_row_range().start));
        if self.hide {
            let col = canvas.frame_size().cols - Self::HIDE_COLS;
            canvas.set_col_offset(col);
            canvas.drawln(Token::new("└─ s(H)ow ─"));
        } else {
            let col = canvas.frame_size().cols - Self::SHOW_COLS;
            canvas.set_col_offset(col);
            canvas.drawln(Token::new("│ (q)uit [ESC,C-c]"));
            if tree.cursor_row() != 0 {
                canvas.drawln(Token::new("│ (r)ecenter [C-l]"));
            }
            if tree.can_cursor_up() {
                canvas.drawln(Token::new("│ (↑)      [k,C-p]"));
            }
            if tree.can_cursor_down() {
                canvas.drawln(Token::new("│ (↓)      [j,C-n]"));
            }
            if tree.can_cursor_left() {
                canvas.drawln(Token::new("│ (←)      [h,C-b]"));
            }
            if tree.can_cursor_right() {
                canvas.drawln(Token::new("│ (→)      [l,C-f]"));
            }
            if tree.can_toggle() {
                canvas.drawln(Token::new("│ (t)oggle   [TAB]"));
            }
            if tree.can_stage_or_discard() {
                canvas.drawln(Token::new("│ (s)tage         "));
            }
            if tree.can_stage_or_discard() {
                canvas.drawln(Token::new("│ (D)iscard       "));
            }
            if tree.can_unstage() {
                canvas.drawln(Token::new("│ (u)nstage       "));
            }
            canvas.drawln(Token::new("└──── (H)ide ─────"));
        }
    }

    pub fn toggle_hide(&mut self) {
        self.hide = !self.hide;
    }
}
