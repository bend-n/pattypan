// one-indexed cell array
pub struct Cells {
    pub size: (u16, u16),
    pub cells: Vec<Cell>,
    pub row: u16,
}
#[derive(Clone, Copy, Debug)]
pub struct Style {
    pub bg: [u8; 3],
    pub color: [u8; 3],
    pub flags: u8,
}
use std::default::Default::default;
use std::iter::{empty, repeat, repeat_n};

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

#[derive(Clone, Copy, Default, Debug)]
pub struct Cell {
    pub style: Style,
    pub letter: Option<char>,
}

impl Cells {
    pub fn new((c, r): (u16, u16)) -> Self {
        let (c, r) = (c + 1, r + 1);
        Self {
            size: (c, r),
            cells: vec![Cell::default(); (c as usize) * (r as usize)],
            row: 0,
        }
    }
    fn offset(&mut self) -> usize {
        self.row as usize * self.size.0 as usize
    }
    pub fn cells(&mut self) -> &mut [Cell] {
        let o = self.offset();
        assert!(
            self.cells.len()
                == o + (self.size.0 as usize) * (self.size.1 as usize)
        );
        &mut self.cells[o..]
    }
    pub fn r(&mut self) -> u16 {
        self.size.1
    }
    pub fn c(&mut self) -> u16 {
        self.size.0
    }
    #[track_caller]
    fn at(&mut self, (x, y): (u16, u16)) -> usize {
        self.offset() + ((y - 1) * self.c() + x - 1) as usize
    }
    #[track_caller]
    pub fn get_at(&mut self, (x, y): (u16, u16)) -> &mut Cell {
        assert!(x < self.c() && y < self.r(), "out of bounds");
        let i = self.at((x, y));
        &mut self.cells[i]
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
        &mut self.rows().nth(row as usize - 1).unwrap()[x as usize - 1..]
    }
    pub fn grow(&mut self, by: u16) {
        self.row += by;
        self.cells.extend(repeat_n(
            Cell::default(),
            by as usize * (self.size.0) as usize,
        ));
    }

    pub fn insert_chars(&mut self, characters: u16, (x, y): (u16, u16)) {
        let s = &mut self.row(y)[x as usize - 1..];
        s.rotate_right(characters as usize);
        s[..characters as usize].fill(Cell::default());
        let w = self.c();
        self.row(y)[w as usize - characters as usize..]
            .fill(Cell::default());
    }
    pub fn insert_lines(&mut self, lines: u16, below: u16) {
        let c = self.c();
        let o = self.offset();
        let next_row = o + (c * (below - 1)) as usize;
        self.cells
            .splice(
                next_row..next_row,
                repeat_n(default(), (lines * c) as usize),
            )
            .for_each(drop);
        self.cells
            .drain(o + self.size.0 as usize * self.size.1 as usize..)
            .for_each(drop);
    }
}
