// one-indexed cell array
pub struct Cells {
    pub size: (u16, u16),
    pub cells: Vec<Cell>,
    pub margin: (u16, u16),
    pub row: u16,
}
#[derive(Clone, Copy, Debug)]
pub struct Style {
    pub bg: [u8; 3],
    pub color: [u8; 3],
    pub flags: u8,
}
use std::default::Default::default;
use std::fmt::Debug;
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

#[derive(Clone, Copy, Default)]
pub struct Cell {
    pub style: Style,
    pub letter: Option<char>,
}
impl Debug for Cell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.letter.unwrap_or(' '))
    }
}
impl Debug for Cells {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Cells")
            .field("size", &self.size)
            .field("margin", &self.margin)
            .field("row", &self.row)
            .field_with("cells", |x| {
                for r in self.cells
                    [{ self.row as usize * self.size.0 as usize }..]
                    .chunks_exact(self.size.0 as _)
                {
                    writeln!(x, "{r:?}")?;
                }
                Ok(())
            })
            .finish()
    }
}
impl Cell {
    pub fn basic(c: char) -> Self {
        Self {
            letter: Some(c),
            ..default()
        }
    }
}

impl Cells {
    pub fn new((c, r): (u16, u16)) -> Self {
        let (c, r) = (c + 1, r + 1);
        Self {
            size: (c, r),
            margin: (1, r),
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
        let at = self.offset()
            + (self.margin.1 as usize - 1) * self.size.0 as usize;
        self.cells.splice(
            at..at,
            repeat_n(
                Cell::default(),
                by as usize * (self.size.0) as usize,
            ),
        );
    }
    pub fn insert_chars(&mut self, characters: u16, (x, y): (u16, u16)) {
        let s = &mut self.row(y)[x as usize - 1..];
        s.rotate_right(characters as usize);
        s[..characters as usize].fill(Cell::default());
        let w = self.c();
        self.row(y)[w as usize - characters as usize..]
            .fill(Cell::default());
    }
    pub fn delete_chars(&mut self, characters: u16, (x, y): (u16, u16)) {
        let s = &mut self.row(y)[x as usize - 1..];
        s.rotate_left(characters as usize);
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
        // let at = self.offset() + self.margin.1 as usize;
        // self.cells
        //     .drain(
        //         (o + self.size.0 as usize * self.size.1 as usize) - at..at,
        //     )
        //     .for_each(drop);
    }
}

#[cfg(test)]
mod test {
    use crate::term::cells::{Cell, Cells};

    #[test]
    fn grow() {
        let mut cells = Cells::new((4, 4));
        cells.row(1).fill(Cell::basic('1'));
        cells.row(2).fill(Cell::basic('2'));
        cells.row(3).fill(Cell::basic('3'));
        cells.row(4).fill(Cell::basic('4'));
        cells.row(5).fill(Cell::basic('5'));
        dbg!(&cells);

        // cells.margin.1 = 3;
        cells.grow(1);
        dbg!(&cells);
        assert_eq!(
            format!("{cells:?}"),
            "Cells { size: (5, 5), margin: (1, 5), row: 1, cells: [2, 2, 2, 2, 2]\n[3, 3, 3, 3, 3]\n[4, 4, 4, 4, 4]\n[5, 5, 5, 5, 5]\n[ ,  ,  ,  ,  ]\n }"
        );
    }
}
