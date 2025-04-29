use std::{num::NonZeroUsize, ops::Range};

use tuinix::{TerminalFrame, TerminalPosition, TerminalSize, TerminalStyle};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

#[derive(Debug)]
pub struct Canvas {
    frame: Frame,
    frame_row_offset: usize,
    cursor: TerminalPosition,
    col_offset: usize,
}

impl Canvas {
    pub fn new(frame_row_offset: usize, frame_size: TerminalSize) -> Self {
        Self {
            frame: Frame::new(frame_size),
            frame_row_offset,
            cursor: TerminalPosition::ZERO,
            col_offset: 0,
        }
    }

    pub fn frame_row_range(&self) -> Range<usize> {
        Range {
            start: self.frame_row_offset,
            end: self.frame_row_offset + self.frame.size.rows,
        }
    }

    pub fn frame_size(&self) -> TerminalSize {
        self.frame.size
    }

    pub fn is_frame_exceeded(&self) -> bool {
        self.cursor.row >= self.frame_row_range().end
    }

    pub fn cursor(&self) -> TerminalPosition {
        self.cursor
    }

    pub fn set_cursor(&mut self, position: TerminalPosition) {
        self.cursor = position;
    }

    pub fn set_col_offset(&mut self, offset: usize) {
        self.col_offset = offset;
    }

    pub fn draw(&mut self, token: Token) {
        let cols = token.cols();
        self.draw_at(self.cursor, token);
        self.cursor.col += cols;
    }

    pub fn drawln(&mut self, token: Token) {
        self.draw(token);
        self.newline();
    }

    pub fn newline(&mut self) {
        self.cursor.row += 1;
        self.cursor.col = 0;
    }

    pub fn draw_at(&mut self, mut position: TerminalPosition, token: Token) {
        if !self.frame_row_range().contains(&position.row) {
            return;
        }

        position.col += self.col_offset;

        let i = position.row - self.frame_row_offset;
        let line = &mut self.frame.lines[i];
        line.draw_token(position.col, token);
        line.split_off(self.frame.size.cols);
    }

    pub fn into_frame(self) -> TerminalFrame {
        //self.frame
        todo!()
    }
}

#[derive(Debug, Clone)]
pub struct Frame {
    size: TerminalSize,
    lines: Vec<FrameLine>,
}

impl Frame {
    pub fn new(size: TerminalSize) -> Self {
        Self {
            size,
            lines: vec![FrameLine::new(); size.rows],
        }
    }

    pub fn dirty_lines<'a>(
        &'a self,
        prev: &'a Self,
    ) -> impl 'a + Iterator<Item = (usize, &'a FrameLine)> {
        self.lines
            .iter()
            .zip(prev.lines.iter())
            .enumerate()
            .filter_map(|(i, (l0, l1))| (l0 != l1).then_some((i, l0)))
            .chain(self.lines.iter().enumerate().skip(prev.lines.len()))
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct FrameLine {
    tokens: Vec<Token>,
}

impl FrameLine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn tokens(&self) -> &[Token] {
        &self.tokens
    }

    pub fn text(&self) -> String {
        self.tokens.iter().map(|t| t.text.clone()).collect()
    }

    pub fn draw_token(&mut self, col: usize, token: Token) {
        if let Some(n) = col.checked_sub(self.cols()).and_then(NonZeroUsize::new) {
            let s: String = std::iter::repeat_n(' ', n.get()).collect();
            self.tokens.push(Token::new(s));
        }

        let mut suffix = self.split_off(col);
        let suffix = suffix.split_off(token.cols());
        self.tokens.push(token);
        self.tokens.extend(suffix.tokens);
    }

    fn split_off(&mut self, col: usize) -> Self {
        let mut acc_cols = 0;
        for i in 0..self.tokens.len() {
            if acc_cols == col {
                let suffix = self.tokens.split_off(i);
                return Self { tokens: suffix };
            }

            let token_cols = self.tokens[i].cols();
            acc_cols += token_cols;
            if acc_cols == col {
                continue;
            } else if let Some(n) = acc_cols.checked_sub(col) {
                let mut suffix = self.tokens.split_off(i);
                let token_prefix_cols = token_cols - n;
                let token_prefix = suffix[0].split_prefix_off(token_prefix_cols);
                self.tokens.push(token_prefix);
                return Self { tokens: suffix };
            }
        }

        // `col` is out of range, so no splitting is needed.
        Self::new()
    }

    pub fn cols(&self) -> usize {
        self.tokens.iter().map(|t| t.cols()).sum()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    text: String,
    style: TerminalStyle,
}

impl Token {
    pub fn new(text: impl Into<String>) -> Self {
        Self::with_style(text, TerminalStyle::new())
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn style(&self) -> TerminalStyle {
        self.style
    }

    pub fn with_style(text: impl Into<String>, style: TerminalStyle) -> Self {
        let mut text = text.into();
        if text.chars().any(|c| c.is_control()) {
            let mut escaped_text = String::new();
            for c in text.chars() {
                if c.is_control() {
                    escaped_text.extend(c.escape_default());
                } else {
                    escaped_text.push(c);
                }
            }
            text = escaped_text;
        }
        Self { text, style }
    }

    pub fn split_prefix_off(&mut self, col: usize) -> Self {
        let mut acc_cols = 0;
        for (i, c) in self.text.char_indices() {
            if acc_cols == col {
                let suffix = self.text.split_off(i);
                return std::mem::replace(self, Self::with_style(suffix, self.style));
            }

            let next_acc_cols = acc_cols + c.width().expect("infallible");
            if next_acc_cols > col {
                // Not a char boundary.
                let suffix = self.text.split_off(i + c.len_utf8());
                let suffix = Self::with_style(suffix, self.style);
                let _ = self.text.pop();
                for _ in acc_cols..col {
                    self.text.push('…');
                }
                return std::mem::replace(self, suffix);
            }
            acc_cols = next_acc_cols;
        }

        std::mem::replace(self, Self::with_style(String::new(), self.style))
    }

    pub fn cols(&self) -> usize {
        self.text.width()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_line() -> orfail::Result<()> {
        let mut line = FrameLine::new();

        line.draw_token(2, Token::new("foo"));
        assert_eq!(line.text(), "  foo");

        line.draw_token(4, Token::new("bar"));
        assert_eq!(line.text(), "  fobar");

        line.draw_token(7, Token::new("baz"));
        assert_eq!(line.text(), "  fobarbaz");

        line.draw_token(6, Token::new("qux"));
        assert_eq!(line.text(), "  fobaquxz");

        // Control chars are escaped.
        line.draw_token(0, Token::new("0\n1"));
        assert_eq!(line.text(), "0\\n1baquxz");

        Ok(())
    }
}
