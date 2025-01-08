use std::{num::NonZeroUsize, ops::Range};

use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::terminal::TerminalSize;

// TODO: rename
#[derive(Debug)]
pub struct Canvas2 {
    // TODO: private
    pub frame: Frame,
    pub frame_row_offset: usize,
}

impl Canvas2 {
    pub fn new(frame_size: TerminalSize) -> Self {
        Self {
            frame: Frame::new(frame_size),
            frame_row_offset: 0,
        }
    }

    pub fn frame_row_range(&self) -> Range<usize> {
        Range {
            start: self.frame_row_offset,
            end: self.frame_row_offset + self.frame.size.rows,
        }
    }

    pub fn draw_token(&mut self, position: CanvasPosition, token: Token) {
        if !self.frame_row_range().contains(&position.col) {
            return;
        }

        let i = position.row - self.frame_row_offset;
        let line = &mut self.frame.lines[i];
        line.draw_token(position.col, token);
    }

    pub fn take_frame(&mut self) -> Frame {
        let size = self.frame.size;
        std::mem::replace(&mut self.frame, Frame::new(size))
    }
}

#[derive(Debug, Clone)]
pub struct Frame {
    // TODO: private
    pub size: TerminalSize,
    pub lines: Vec<FrameLine>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenStyle {
    Plain,
    Bold,
    Dim,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub text: String,
    pub style: TokenStyle,
}

impl Token {
    pub fn plain(s: impl Into<String>) -> Self {
        // TODO: replace invalid chars with '?'
        Self {
            text: s.into(),
            style: TokenStyle::Plain,
        }
    }

    pub fn split_prefix_off(&mut self, col: usize) -> Self {
        // TODO: refactor
        let mut acc_cols = 0;
        for (i, c) in self.text.char_indices() {
            if acc_cols == col {
                let suffix = self.text.split_off(i);
                let suffix = Self {
                    text: suffix,
                    style: self.style,
                };
                return std::mem::replace(self, suffix);
            }

            let next_acc_cols = acc_cols + c.width().expect("infallible");
            if next_acc_cols > col {
                // Not a char boundary.
                let suffix = self.text.split_off(i + c.len_utf8());
                let suffix = Self {
                    text: suffix,
                    style: self.style,
                };

                let _ = self.text.pop();
                for _ in acc_cols..col {
                    self.text.push('…');
                }

                return std::mem::replace(self, suffix);
            }
            acc_cols = next_acc_cols;
        }

        std::mem::replace(
            self,
            Self {
                text: String::new(),
                style: self.style,
            },
        )
    }

    pub fn cols(&self) -> usize {
        self.text.width()
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct FrameLine {
    pub tokens: Vec<Token>,
}

impl FrameLine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn draw_token(&mut self, col: usize, token: Token) {
        if let Some(n) = col.checked_sub(self.cols()).and_then(NonZeroUsize::new) {
            let s: String = std::iter::repeat_n(' ', n.get()).collect();
            self.tokens.push(Token::plain(s));
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

            acc_cols += self.tokens[i].cols();
            if acc_cols == col {
                continue;
            } else if let Some(n) = acc_cols.checked_sub(col) {
                let mut suffix = self.tokens.split_off(i);
                let token_prefix = suffix[0].split_prefix_off(n);
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

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct CanvasPosition {
    pub row: usize,
    pub col: usize,
}

impl CanvasPosition {
    pub fn row_col(row: usize, col: usize) -> Self {
        Self { row, col }
    }
}

#[derive(Debug, Default)]
pub struct Canvas {
    current_row: Row,
    pub rows: Vec<Row>, // TODO:private
    max_cols: usize,
}

impl Canvas {
    pub fn new(max_cols: usize) -> Self {
        Self {
            current_row: Row::default(),
            rows: Vec::new(),
            max_cols,
        }
    }

    pub fn clip(&mut self, offset: usize, rows: usize) {
        self.rows.drain(..offset);
        self.rows.truncate(rows);
    }

    pub fn draw_canvas(&mut self, position: Position, canvas: Canvas) {
        for (row_i, src_row) in canvas.rows.into_iter().enumerate() {
            let row_i = row_i + position.row;
            while self.rows.len() <= row_i {
                self.rows.push(Row::default());
            }

            let dst_row = &mut self.rows[row_i];
            dst_row.replace(position.col, src_row);
        }
    }

    pub fn draw_text(&mut self, text: Text) {
        self.current_row.texts.push(text);
    }

    pub fn draw_textl(&mut self, text: Text) {
        self.draw_text(text);
        self.draw_newline();
    }

    pub fn draw_newline(&mut self) {
        let mut last_row = std::mem::take(&mut self.current_row);

        // TODO: refactor
        last_row.truncate(self.max_cols);

        self.rows.push(last_row);
    }

    pub fn rows(&self) -> usize {
        self.rows.len() + 1
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Row {
    pub texts: Vec<Text>,
}

impl Row {
    pub fn replace(&mut self, col: usize, src: Row) {
        self.truncate(col);
        let n = self.cols();
        if col > n {
            let mut padding = String::new();
            for _ in n..col {
                padding.push(' ');
            }
            self.texts.push(Text::new(&padding).expect("infallible"));
        }
        self.texts.extend(src.texts);
    }

    pub fn truncate(&mut self, max_cols: usize) {
        let mut acc_cols = 0;
        for text in &mut self.texts {
            let text_cols = text.cols();
            if acc_cols + text_cols < max_cols {
                acc_cols += text_cols;
                continue;
            }
            text.truncate(max_cols - acc_cols);
        }
    }

    pub fn cols(&self) -> usize {
        self.texts.iter().map(|x| x.cols()).sum()
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub row: usize,
    pub col: usize,
}

impl Position {
    pub fn new(row: usize, col: usize) -> Self {
        Self { row, col }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Text {
    pub text: String,                        // TODO: private
    pub attrs: crossterm::style::Attributes, // TODO: private
}

impl Text {
    pub fn new(text: &str) -> orfail::Result<Self> {
        // TODO: validate or replace non visible chars
        Ok(Self {
            text: text.to_owned(),
            attrs: crossterm::style::Attributes::default(),
        })
    }

    pub fn cols(&self) -> usize {
        self.text.width()
    }

    pub fn truncate(&mut self, max_cols: usize) {
        let mut cols = 0;
        for (i, c) in self.text.char_indices() {
            cols += c.width().expect("infallible");
            if cols >= max_cols {
                self.text.truncate(i);
                break;
            }
        }
    }

    pub fn bold(mut self) -> Self {
        self.attrs.set(crossterm::style::Attribute::Bold);
        self
    }

    pub fn dim(mut self) -> Self {
        self.attrs.set(crossterm::style::Attribute::Dim);
        self
    }
}
