use std::iter::repeat_n;

use ctlfun::Parameter::*;
use ctlfun::TerminalInput::*;
use ctlfun::{ControlFunction, TerminalInputParser};

pub struct Terminal {
    pub style: Style,
    pub cursor: (u16, u16),
    pub size: (u16, u16),

    pub cells: Vec<Cell>,
    pub row: usize,
    pub p: TerminalInputParser,
    pub mode: Mode,
}
pub enum Mode {
    Normal,
    Raw,
}

#[derive(Clone, Copy)]
pub struct Style {
    pub bg: [u8; 3],
    pub color: [u8; 3],
    pub style: u8,
}
use crate::colors;
impl std::default::Default for Style {
    fn default() -> Self {
        Self {
            bg: colors::BACKGROUND,
            style: 0,
            color: colors::FOREGROUND,
        }
    }
}

#[derive(Clone, Copy, Default)]
pub struct Cell {
    pub style: Style,
    pub letter: Option<char>,
}

impl Terminal {
    #[implicit_fn::implicit_fn]
    pub fn rx(&mut self, x: u8) {
        match self.p.parse_byte(x) {
            Continue => {}
            Char(x) => {
                dbg!(x);
                self.cursor.0 += 1;
                if self.cursor.0 == self.size.0 {
                    println!("overflow");
                    self.cursor.0 = 1;
                    self.cursor.1 += 1;
                }
                while self.cursor.1 >= self.size.1 {
                    println!("newline");
                    self.cursor.1 -= 1;
                    // self.cells
                    //     .drain(..self.size.0 as usize)
                    //     .for_each(drop);
                    self.row += 1;
                    self.cells.extend(repeat_n(
                        Cell::default(),
                        self.size.0 as usize,
                    ));
                }
                assert!(
                    self.cells.len()
                        == self.row * self.size.0 as usize
                            + self.size.0 as usize * self.size.1 as usize
                );
                assert!(
                    self.cells[self.row * self.size.0 as usize..].len()
                        == self.size.0 as usize * self.size.1 as usize
                );
                dbg!(self.cursor, self.size);
                let c = &mut self.cells[self.row * self.size.0 as usize..]
                    // y*w+x
                    [(self.cursor.1 * self.size.0 + self.cursor.0)
                        as usize];
                c.letter = Some(x);
                c.style = self.style;
            }
            Control(ControlFunction { start: 8, .. }) => {
                self.cursor.0 -= 1;
            }
            Control(ControlFunction {
                start: b'[',
                params,
                end: b'm',
                ..
            }) => match params {
                &[Value(0)] => self.style = Style::default(),
                &[Value(x @ (30..=37))] => {
                    self.style.color = colors::FOUR[x as usize - 30]
                }
                &[Value(39)] => self.style.color = colors::FOREGROUND,
                &[Value(x @ (40..=47))] => {
                    self.style.bg = colors::FOUR[x as usize - 40]
                }
                &[Value(49)] => self.style.bg = colors::BACKGROUND,
                &[Value(x @ (90..=97))] => {
                    self.style.color = colors::FOUR[x as usize - 82]
                }
                &[Value(x @ (100..=107))] => {
                    self.style.bg = colors::FOUR[x as usize - 92]
                }
                &[Value(38), Value(2), Value(r), Value(g), Value(b)] => {
                    self.style.color =
                        [r, g, b].map(|x| x.min(0xff) as u8);
                }
                &[Value(48), Value(2), Value(r), Value(g), Value(b)] => {
                    self.style.bg = [r, g, b].map(|x| x.min(0xff) as u8);
                }
                &[
                    Value(38),
                    Value(2),
                    Value(r),
                    Value(g),
                    Value(b),
                    Value(48),
                    Value(2),
                    Value(rb),
                    Value(gb),
                    Value(bb),
                ] => {
                    self.style.bg =
                        [rb, gb, bb].map(|x| x.min(0xff) as u8);
                    self.style.color =
                        [r, g, b].map(|x| x.min(0xff) as u8);
                }
                _ => {}
            },
            Control(ControlFunction {
                start: b'[',
                params,
                end: b'A',
                ..
            }) if let [p] = params => {
                self.cursor.1 -= p.value_or(1);
            }
            Control(ControlFunction {
                start: b'[',
                params,
                end: b'B',
                ..
            }) if let [p] = params => {
                self.cursor.1 += p.value_or(1);
            }
            Control(ControlFunction {
                start: b'[',
                params,
                end: b'C',
                ..
            }) if let [p] = params => {
                self.cursor.0 += p.value_or(1);
            }
            Control(ControlFunction {
                start: b'[',
                params,
                end: b'D',
                ..
            }) if let [p] = params => {
                self.cursor.0 -= p.value_or(1);
            }
            Control(ControlFunction {
                start: b'[',
                params,
                end: b'K',
                ..
            }) if params == &[Default] => {
                for cell in &mut self.cells
                    [self.row * self.size.0 as usize..]
                    [(self.cursor.1 * self.size.0 + self.cursor.0 + 1)
                        as usize
                        ..(self.cursor.1 * self.size.0 + self.size.0)
                            as usize]
                // [self.cursor.1 as usize..self.size.0 as usize]
                // [self.cursor.0 as usize..]
                {
                    cell.letter = None;
                }
            }
            Control(ControlFunction { start: b'\r', .. }) => {
                self.cursor.0 = 1;
            }
            Control(ControlFunction { start: b'\n', .. }) => {
                self.cursor.1 += 1;
            }
            Control(x) => {
                dbg!(x);
                println!(
                    "{} {:?} {} {}",
                    x.start as char,
                    x.params
                        .iter()
                        .map(|x| match x {
                            ctlfun::Parameter::Default =>
                                "default".to_string(),
                            ctlfun::Parameter::Value(x) => x.to_string(),
                        })
                        .collect::<Vec<_>>(),
                    String::from_utf8_lossy(&x.bytes),
                    x.end as char,
                );
            }
            _ => unreachable!(),
        }
    }
}
