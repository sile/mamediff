use crate::{
    app::App,
    canvas::{Canvas, Token, TokenPosition},
};

#[derive(Debug, Default)]
pub struct LegendWidget {
    pub hide: bool,
}

impl LegendWidget {
    const SHOW_COLS: usize = 19;
    const HIDE_COLS: usize = 11;

    // TODO: s/App/TreeNodeWidget/
    pub fn render(&self, canvas: &mut Canvas, app: &App) {
        if canvas.frame_size().cols <= Self::SHOW_COLS {
            return;
        }

        canvas.set_cursor(TokenPosition::row(canvas.frame_row_range().start));
        if self.hide {
            let col = canvas.frame_size().cols - Self::HIDE_COLS;
            canvas.set_col_offset(col);
            canvas.drawl(Token::new("|   ...    "));
            canvas.drawl(Token::new("+- s(h)ow -"));
        } else {
            let col = canvas.frame_size().cols - Self::SHOW_COLS;
            canvas.set_col_offset(col);
            canvas.drawl(Token::new("| (q)uit [ESC,C-c]"));

            if app.cursor.path.last() != Some(&0) {
                canvas.drawl(Token::new("| (↑)        [C-p]"));
            }
            if app.can_down() {
                canvas.drawl(Token::new("| (↓)        [C-n]"));
            }
            if app.cursor.path.len() > 2 {
                canvas.drawl(Token::new("| (←)        [C-f]"));
            }
            if app.can_right() {
                canvas.drawl(Token::new("| (→)        [C-b]"));
            }
            if app.is_togglable() {
                canvas.drawl(Token::new("| (t)oggle   [TAB]"));
            }
            if app.can_stage() {
                canvas.drawl(Token::new("| (s)tage         "));
            }
            if app.can_stage() {
                canvas.drawl(Token::new("| (D)iscard       "));
            }
            if app.can_unstage() {
                canvas.drawl(Token::new("| (u)nstage       "));
            }
            canvas.drawl(Token::new("+---- (h)ide -----"));
        }
    }

    pub fn toggle_hide(&mut self) {
        self.hide = !self.hide;
    }
}
