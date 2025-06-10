// one-indexed cell array
pub struct Cells {
    pub size: (u16, u16),
    pub cells: Vec<Cell>,
    pub row: u16,
}
#[derive(Clone, Copy)]
pub struct Style {
    pub bg: [u8; 3],
    pub color: [u8; 3],
    pub flags: u8,
}
use std::iter::repeat_n;

use crate::colors;
impl std::default::Default for Style {
    fn default() -> Self {
        Self {
            bg: colors::BACKGROUND,
            flags: 0,
            color: colors::FOREGROUND,
        }
    }
}

#[derive(Clone, Copy, Default)]
pub struct Cell {
    pub style: Style,
    pub letter: Option<char>,
}

impl Cells {
    pub fn new((c, r): (u16, u16)) -> Self {
        Self {
            size: (c + 1, r + 1),
            cells: vec![
                Cell::default();
                (c as usize + 1) * (r as usize + 1)
            ],
            row: 0,
        }
    }
    pub fn cells(&mut self) -> &mut [Cell] {
        assert!(
            self.cells.len()
                == self.row as usize * (self.size.0 as usize)
                    + (self.size.0 as usize) * (self.size.1 as usize)
        );
        assert!(
            self.cells[self.row as usize * self.size.0 as usize..].len()
                == self.size.0 as usize * self.size.1 as usize
        );
        &mut self.cells[self.row as usize * self.size.0 as usize..]
    }
    pub fn r(&mut self) -> u16 {
        self.size.1
    }
    pub fn c(&mut self) -> u16 {
        self.size.0
    }
    #[track_caller]
    pub fn get_at(&mut self, (x, y): (u16, u16)) -> &mut Cell {
        let w = self.c();
        assert!(x < self.c() && y < self.r(), "out of bounds");
        &mut self.cells()[((y - 1) * w + x - 1) as usize]
    }
    pub fn rows(&mut self) -> impl Iterator<Item = &mut [Cell]> {
        let w = self.c();
        self.cells().chunks_exact_mut(w as _)
    }
    pub fn row(&mut self, row: u16) -> &mut [Cell] {
        let w = self.c();
        &mut self.cells()
            [(row as usize - 1) * w as usize..row as usize * w as usize]
    }
    pub fn past(&mut self, (x, row): (u16, u16)) -> &mut [Cell] {
        &mut self.rows().nth(row as usize - 1).unwrap()[x as usize..]
    }
    pub fn grow(&mut self, by: u16) {
        self.row += by;
        self.cells.extend(repeat_n(
            Cell::default(),
            by as usize * (self.size.0) as usize,
        ));
    }
}
