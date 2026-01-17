use vt100::Parser;

pub struct Screen {
    parser: Parser,
}

impl Screen {
    pub fn new(rows: usize, cols: usize) -> Self {
        Self {
            parser: Parser::new(rows as u16, cols as u16, 0),
        }
    }

    pub fn resize(&mut self, rows: usize, cols: usize) {
        self.parser.screen_mut().set_size(rows as u16, cols as u16);
    }

    pub fn process(&mut self, bytes: &[u8]) {
        self.parser.process(bytes);
    }

    pub fn full_text(&self) -> String {
        self.parser.screen().contents()
    }
}
