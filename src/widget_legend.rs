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
            canvas.drawl(Token::new("+- s(h)ow -"));
        } else {
            let col = canvas.frame_size().cols - Self::SHOW_COLS;
            canvas.set_col_offset(col);
            canvas.drawl(Token::new("| (q)uit [ESC,C-c]"));
            if tree.cursor_row() != 0 {
                canvas.drawl(Token::new("| (r)ecenter [C-l]"));
            }
            if tree.can_cursor_up() {
                canvas.drawl(Token::new("| (↑)        [C-p]"));
            }
            if tree.can_cursor_down() {
                canvas.drawl(Token::new("| (↓)        [C-n]"));
            }
            if tree.can_cursor_left() {
                canvas.drawl(Token::new("| (←)        [C-b]"));
            }
            if tree.can_cursor_right() {
                canvas.drawl(Token::new("| (→)        [C-f]"));
            }
            if tree.can_toggle() {
                canvas.drawl(Token::new("| (t)oggle   [TAB]"));
            }
            if tree.can_stage_or_discard() {
                canvas.drawl(Token::new("| (s)tage         "));
            }
            if tree.can_stage_or_discard() {
                canvas.drawl(Token::new("| (D)iscard       "));
            }
            if tree.can_unstage() {
                canvas.drawl(Token::new("| (u)nstage       "));
            }
            canvas.drawl(Token::new("+---- (h)ide -----"));
        }
    }

    pub fn toggle_hide(&mut self) {
        self.hide = !self.hide;
    }
}
