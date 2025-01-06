#[derive(Debug, Default)]
pub struct Canvas {
    current_row: Row,
    pub rows: Vec<Row>, // TODO:private
}

impl Canvas {
    pub fn new() -> Self {
        Self {
            current_row: Row::default(),
            rows: Vec::new(),
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
        let last_row = std::mem::take(&mut self.current_row);
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
        // TODO: consider multi byte char
        let mut n = 0;
        for text in &mut self.texts {
            if n + text.text.len() < col {
                n += text.text.len();
                continue;
            }
            text.text.truncate(col - n);
            n = col;
        }
        if col > n {
            let mut padding = String::new();
            for _ in n..col {
                padding.push(' ');
            }
            self.texts.push(Text::new(&padding).expect("infallible"));
        }
        self.texts.extend(src.texts);
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
        // TODO: validate
        Ok(Self {
            text: text.to_owned(),
            attrs: crossterm::style::Attributes::default(),
        })
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
