use vte::Perform;

#[derive(Debug, Clone)]
pub struct Screen {
    rows: usize,
    cols: usize,
    grid: Vec<Vec<char>>,
    cursor_row: usize,
    cursor_col: usize,
}

impl Screen {
    pub fn new(rows: usize, cols: usize) -> Self {
        Self {
            rows,
            cols,
            grid: vec![vec![' '; cols]; rows],
            cursor_row: 0,
            cursor_col: 0,
        }
    }

    pub fn resize(&mut self, rows: usize, cols: usize) {
        let mut new_grid = vec![vec![' '; cols]; rows];
        let min_rows = rows.min(self.rows);
        let min_cols = cols.min(self.cols);
        for r in 0..min_rows {
            for c in 0..min_cols {
                new_grid[r][c] = *self.grid.get(r).and_then(|row| row.get(c)).unwrap_or(&' ');
            }
        }
        self.rows = rows;
        self.cols = cols;
        self.grid = new_grid;
        self.cursor_row = self.cursor_row.min(self.rows.saturating_sub(1));
        self.cursor_col = self.cursor_col.min(self.cols.saturating_sub(1));
    }

    fn scroll_up(&mut self, lines: usize) {
        for _ in 0..lines {
            self.grid.remove(0);
            self.grid.push(vec![' '; self.cols]);
        }
        self.cursor_row = self.cursor_row.saturating_sub(lines);
    }

    fn clear_screen(&mut self) {
        for row in &mut self.grid {
            for cell in row {
                *cell = ' ';
            }
        }
        self.cursor_row = 0;
        self.cursor_col = 0;
    }

    fn clear_line_from_cursor(&mut self) {
        if self.cursor_row < self.rows {
            for c in self.cursor_col..self.cols {
                self.grid[self.cursor_row][c] = ' ';
            }
        }
    }

    fn set_cursor(&mut self, row: usize, col: usize) {
        self.cursor_row = row.min(self.rows.saturating_sub(1));
        self.cursor_col = col.min(self.cols.saturating_sub(1));
    }

    pub fn full_text(&self) -> String {
        self.grid
            .iter()
            .map(|row| {
                let mut s: String = row.iter().collect();
                while s.ends_with(' ') {
                    s.pop();
                }
                s
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

pub struct ScreenPerformer<'a> {
    screen: &'a mut Screen,
}

impl<'a> ScreenPerformer<'a> {
    pub fn new(screen: &'a mut Screen) -> Self {
        Self { screen }
    }
}

impl<'a> Perform for ScreenPerformer<'a> {
    fn print(&mut self, c: char) {
        if self.screen.cursor_row >= self.screen.rows {
            self.screen.scroll_up(1);
            self.screen.cursor_row = self.screen.rows.saturating_sub(1);
        }
        if self.screen.cursor_col >= self.screen.cols {
            self.screen.cursor_col = 0;
            self.screen.cursor_row += 1;
            if self.screen.cursor_row >= self.screen.rows {
                self.screen.scroll_up(1);
                self.screen.cursor_row = self.screen.rows.saturating_sub(1);
            }
        }
        if self.screen.cursor_row < self.screen.rows && self.screen.cursor_col < self.screen.cols {
            self.screen.grid[self.screen.cursor_row][self.screen.cursor_col] = c;
            self.screen.cursor_col += 1;
        }
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            b'\n' => {
                self.screen.cursor_col = 0;
                self.screen.cursor_row += 1;
                if self.screen.cursor_row >= self.screen.rows {
                    self.screen.scroll_up(1);
                    self.screen.cursor_row = self.screen.rows.saturating_sub(1);
                }
            }
            b'\r' => self.screen.cursor_col = 0,
            b'\x08' => {
                if self.screen.cursor_col > 0 {
                    self.screen.cursor_col -= 1;
                }
            }
            b'\t' => {
                let next_tab = ((self.screen.cursor_col / 8) + 1) * 8;
                self.screen.cursor_col = next_tab.min(self.screen.cols.saturating_sub(1));
            }
            _ => {}
        }
    }

    fn csi_dispatch(
        &mut self,
        params: &vte::Params,
        _intermediates: &[u8],
        _ignore: bool,
        action: char,
    ) {
        let first_param = |idx: usize, default: usize| -> usize {
            params
                .iter()
                .nth(idx)
                .and_then(|p| p.get(0))
                .map(|v| (*v).max(1) as usize)
                .unwrap_or(default)
        };

        match action {
            'A' => {
                let n = first_param(0, 1);
                self.screen.cursor_row = self.screen.cursor_row.saturating_sub(n);
            }
            'B' => {
                let n = first_param(0, 1);
                self.screen.cursor_row = (self.screen.cursor_row + n).min(self.screen.rows - 1);
            }
            'C' => {
                let n = first_param(0, 1);
                self.screen.cursor_col = (self.screen.cursor_col + n).min(self.screen.cols - 1);
            }
            'D' => {
                let n = first_param(0, 1);
                self.screen.cursor_col = self.screen.cursor_col.saturating_sub(n);
            }
            'H' | 'f' => {
                let row = first_param(0, 1);
                let col = first_param(1, 1);
                self.screen.set_cursor(row.saturating_sub(1), col.saturating_sub(1));
            }
            'J' => {
                let mode = first_param(0, 0);
                if mode == 2 {
                    self.screen.clear_screen();
                } else {
                    self.screen.clear_line_from_cursor();
                }
            }
            'K' => self.screen.clear_line_from_cursor(),
            _ => {}
        }
    }

    fn hook(
        &mut self,
        _params: &vte::Params,
        _intermediates: &[u8],
        _ignore: bool,
        _action: char,
    ) {
    }

    fn put(&mut self, _byte: u8) {}

    fn unhook(&mut self) {}
}
