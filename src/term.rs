use std::iter::repeat_n;
use std::ops::Not;

use ctlfun::Parameter::*;
use ctlfun::TerminalInput::*;
use ctlfun::{ControlFunction, TerminalInputParser};

pub struct Terminal {
    pub style: Style,
    pub cursor: (u16, u16),
    pub saved_cursor: (u16, u16),
    pub size: (u16, u16),

    pub cells: Vec<Cell>,
    pub row: usize,
    pub p: TerminalInputParser,
    pub mode: Mode,

    pub alternate: Option<Box<Self>>,
}

use std::default::Default::default;
impl Terminal {
    pub fn new(sz @ (c, r): (u16, u16), alt: bool) -> Self {
        Self {
            style: default(),
            saved_cursor: (1, 1),
            cursor: (1, 1),
            size: sz,
            row: 0,
            cells: vec![
                Cell::default();
                (c as usize + 1) * (r as usize + 1)
            ],
            p: default(),
            mode: Mode::Normal,

            alternate: alt
                .not()
                .then(|| Terminal::new(sz, true))
                .map(Box::new),
        }
    }
}
pub enum Mode {
    Normal,
    Raw,
}

#[derive(Clone, Copy)]
pub struct Style {
    pub bg: [u8; 3],
    pub color: [u8; 3],
    pub flags: u8,
}
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

impl Terminal {
    fn decsc(&mut self) {
        self.saved_cursor = self.cursor;
    }
    fn decrc(&mut self) {
        self.cursor = self.saved_cursor;
    }
    fn clear(&mut self) {
        // TODO
    }
    #[implicit_fn::implicit_fn]
    pub fn rx(&mut self, x: u8) {
        match self.p.parse_byte(x) {
            Continue => {}
            Char(x) => {
                self.cursor.0 += 1;
                if self.cursor.0 == self.size.0 + 1 {
                    println!("overflow");
                    self.cursor.0 = 1;
                    self.cursor.1 += 1;
                }
                while self.cursor.1 > self.size.1 {
                    println!("newline");
                    self.cursor.1 -= 1;
                    self.row += 1;
                    self.cells.extend(repeat_n(
                        Cell::default(),
                        self.size.0 as usize,
                    ));
                }
                // assert!(
                //     self.cells.len()
                //         == self.row * self.size.0 as usize
                //             + self.size.0 as usize * self.size.1 as usize
                // );
                // assert!(
                //     self.cells[self.row * self.size.0 as usize..].len()
                //         == self.size.0 as usize * self.size.1 as usize
                // );
                // dbg!(self.cursor);
                let c = &mut self.cells[self.row * self.size.0 as usize..]
                    // y*w+x
                    [(self.cursor.1 * self.size.0 + self.cursor.0)
                        as usize];
                c.letter = Some(x);
                c.style = self.style;
                dbg!(x);
            }
            Control(ControlFunction {
                start: b'',
                params: [],
                ..
            }) => {
                self.cursor.0 -= 1;
            }
            Control(ControlFunction {
                start: b'[',
                params: [],
                end: b'm',
                ..
            }) => {
                self.style = Style::default();
            }
            Control(ControlFunction {
                start: b'[',
                params,
                end: b'm',
                ..
            }) => {
                let input: Vec<u16> =
                    params.iter().map(_.value_or(0)).collect();
                for &action in parse_sgr()
                    .parse(&input)
                    .into_result()
                    .iter()
                    .flatten()
                {
                    use StyleAction::*;
                    match action {
                        Reset => self.style = Style::default(),
                        SetFg(c) => self.style.color = c,
                        SetBg(c) => self.style.bg = c,
                        ModeSet(1) => self.style.flags |= BOLD,
                        ModeSet(2) => self.style.flags |= DIM,
                        ModeSet(3) => self.style.flags |= ITALIC,
                        ModeSet(4) => self.style.flags |= UNDERLINE,
                        ModeSet(7) => self.style.flags |= INVERT,
                        ModeSet(9) => self.style.flags |= STRIKETHROUGH,
                        ModeSet(22) => self.style.flags &= !(BOLD | DIM),
                        _ => {}
                    }
                }
            }
            Control(ControlFunction {
                start: b'[',
                params: [p],
                end: b'A',
                ..
            }) => {
                self.cursor.1 -= p.value_or(1);
            }
            Control(ControlFunction {
                start: b'[',
                params: [p],
                end: b'B',
                ..
            }) => {
                self.cursor.1 += p.value_or(1);
            }
            Control(ControlFunction {
                start: b'[',
                params: [p],
                end: b'C',
                ..
            }) => {
                self.cursor.0 += p.value_or(1);
            }
            Control(ControlFunction {
                start: b'[',
                params: [p],
                end: b'D',
                ..
            }) => {
                self.cursor.0 -= p.value_or(1);
            }
            Control(ControlFunction {
                start: b'[',
                params: [Value(y), Value(x)],
                end: b'H',
                ..
            }) => {
                self.cursor = (*x, *y);
            }
            Control(ControlFunction {
                start: b'[',
                params: [Default],
                end: b'H',
                ..
            }) => {
                self.cursor = (1, 1);
            }
            Control(ControlFunction {
                start: b'[',
                params: [Value(y)],
                end: b'd',
                ..
            }) => {
                self.cursor.1 = *y;
            }
            Control(ControlFunction {
                start: b'[',
                params: [Value(x)],
                end: b'G',
                ..
            }) => {
                self.cursor.0 = *x;
            }
            Control(ControlFunction {
                start: b'[',
                params: [Default],
                end: b'K',
                ..
            }) => {
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
            Control(ControlFunction {
                start: b'\r',
                params: [],
                ..
            }) => {
                self.cursor.0 = 1;
            }
            Control(ControlFunction {
                start: b'\n',
                params: [],
                ..
            }) => {
                self.cursor.1 += 1;
            }
            Control(
                ControlFunction {
                    start: b'\x1b',
                    params: [],
                    end: b'7',
                    ..
                }
                | ControlFunction {
                    start: b'[',
                    params: [],
                    end: b's',
                    ..
                },
            ) => {
                self.decsc();
            }
            Control(
                ControlFunction {
                    start: b'\x1b',
                    params: [],
                    end: b'8',
                    ..
                }
                | ControlFunction {
                    start: b'[',
                    params: [],
                    end: b'u',
                    ..
                },
            ) => {
                self.decrc();
            }
            Control(ControlFunction {
                start: b'[',
                params: [Value(n)],
                end: b'h' | b'l',
                ..
            }) => match n {
                2004 => {}
                1047 if let Some(mut alt) = self.alternate.take() => {
                    alt.clear();
                    std::mem::swap(self, &mut *alt);
                    self.alternate = Some(alt);
                }
                1048 => self.decsc(),
                1049 if let Some(mut alt) = self.alternate.take() => {
                    alt.clear();
                    self.decsc();
                    std::mem::swap(self, &mut *alt);
                    self.decrc();
                    self.alternate = Some(alt);
                }
                _ => {}
            },
            Control(ControlFunction {
                start: b'\x1b',
                params: [],
                bytes: [b'('],
                end: b'B',
                ..
            }) => {}
            Control(x) => {
                if x.start == b'[' {
                    println!(
                        "CSI {:?} {}",
                        x.params
                            .iter()
                            .map(|x| match x {
                                ctlfun::Parameter::Default =>
                                    "default".to_string(),
                                ctlfun::Parameter::Value(x) =>
                                    x.to_string(),
                            })
                            .collect::<Vec<_>>(),
                        x.end as char,
                    )
                } else {
                    println!(
                        "{} {:?} {} {}",
                        x.start as char,
                        x.params
                            .iter()
                            .map(|x| match x {
                                ctlfun::Parameter::Default =>
                                    "default".to_string(),
                                ctlfun::Parameter::Value(x) =>
                                    x.to_string(),
                            })
                            .collect::<Vec<_>>(),
                        String::from_utf8_lossy(&x.bytes),
                        x.end as char,
                    );
                }
            }
            _ => unreachable!(),
        }
    }
}
#[derive(Clone, Copy, Debug)]
enum StyleAction {
    Reset,
    ModeSet(u16),
    SetFg([u8; 3]),
    SetBg([u8; 3]),
}
pub const BOLD: u8 = 1;
pub const DIM: u8 = 1 << 1;
pub const ITALIC: u8 = 1 << 2;
pub const INVERT: u8 = 1 << 3;
pub const UNDERLINE: u8 = 1 << 4;
pub const STRIKETHROUGH: u8 = 1 << 5;

fn value<'a>(
    r: impl std::ops::RangeBounds<u16>,
) -> impl Parser<'a, &'a [u16], u16> {
    any().filter(move |x| r.contains(x))
}
use chumsky::prelude::*;
#[implicit_fn::implicit_fn]
fn parse_sgr<'a>() -> impl Parser<'a, &'a [u16], Vec<StyleAction>> {
    use StyleAction::*;
    let color = choice((
        // 5;n
        just(5)
            .ignore_then(any())
            .map(|x: u16| colors::EIGHT[x.min(0xff) as usize]),
        just(2)
            .ignore_then(any().repeated().collect_exactly::<[u16; 3]>())
            .map(_.map(|x| x.min(0xff) as u8)),
    ));
    choice((
        just(0u16).to(Reset),
        value(1..10).map(ModeSet),
        value(30..=37).map(_ - 30).map(colors::four).map(SetFg),
        just(38).ignore_then(color.map(SetFg)),
        just(39).to(SetFg(colors::FOREGROUND)),
        value(40..=47).map(_ - 40).map(colors::four).map(SetBg),
        just(48).ignore_then(color.map(SetBg)),
        just(49).to(SetBg(colors::BACKGROUND)),
        value(90..=97).map(_ - 82).map(colors::four).map(SetFg),
        value(100..=107).map(_ - 92).map(colors::four).map(SetBg),
    ))
    .repeated()
    .collect()
    .labelled("style")
}
