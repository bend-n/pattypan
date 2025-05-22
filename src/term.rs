use ctlfun::Parameter::*;
use ctlfun::TerminalInput::*;
use ctlfun::{ControlFunction, TerminalInputParser};

pub struct Terminal {
    pub style: Style,
    pub cursor: (u16, u16),
    pub size: (u16, u16),
    pub scrollback: Scrollback,
    pub cells: Vec<Cell>,
    pub p: TerminalInputParser,
    pub mode: Mode,
}
pub enum Mode {
    Normal,
    Raw,
}
#[derive(Default)]
pub struct Scrollback {
    // invariant: len() / t.size.w == height
    pub history: Vec<Cell>,
    pub height: u16,
}
impl Scrollback {}

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
                let c = &mut self.cells[(self.cursor.1 * self.size.0
                    + self.cursor.0)
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
            }) => match params.get(0).map(*_).unwrap() {
                Value(0) => self.style = Style::default(),
                Value(x @ (30..=37)) => {
                    self.style.color = colors::FOUR[x as usize - 30]
                }
                Value(39) => self.style.color = colors::FOREGROUND,
                Value(x @ (90..=97)) => {
                    self.style.color = colors::FOUR[x as usize - 72]
                }
                _ => {}
            },
            Control(ControlFunction {
                start: b'[',
                params,
                end: b'K',
                ..
            }) if params == &[Default] => {
                for cell in &mut self.cells[(self.cursor.1 * self.size.0
                    + self.cursor.0
                    + 1)
                    as usize
                    ..(self.cursor.1 * self.size.0 + self.size.0) as usize]
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
