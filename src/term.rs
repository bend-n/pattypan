use std::ops::Not;
use std::os::fd::BorrowedFd;
mod cells;
use cells::*;
use ctlfun::Parameter::*;
use ctlfun::TerminalInput::*;
use ctlfun::{ControlFunction, TerminalInputParser};

pub struct Terminal {
    pub style: Style,
    pub cursor: (u16, u16),
    pub saved_cursor: (u16, u16),

    pub cells: Cells,
    pub p: TerminalInputParser,
    pub mode: Mode,

    pub alternate: Option<Box<Self>>,
}

use std::default::Default::default;
impl Terminal {
    pub fn new(sz: (u16, u16), alt: bool) -> Self {
        Self {
            style: default(),
            saved_cursor: (1, 1),
            cursor: (1, 1),
            cells: Cells::new(sz),
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

impl Terminal {
    fn decsc(&mut self) {
        self.saved_cursor = self.cursor;
    }
    fn decrc(&mut self) {
        self.cursor = self.saved_cursor;
    }
    fn clear(&mut self) {
        self.cells.cells().fill(default());
    }
    #[implicit_fn::implicit_fn]
    pub fn rx(&mut self, x: u8, pty: BorrowedFd<'_>) {
        match self.p.parse_byte(x) {
            Continue => {}
            Char(x) => {
                if self.cursor.0 >= self.cells.c() {
                    println!("overflow");
                    self.cursor.0 = 1;
                    self.cursor.1 += 1;
                }
                while self.cursor.1 >= self.cells.r() {
                    println!("newline");
                    self.cursor.1 -= 1;
                    self.cells.grow(1);
                }
                let w = self.cells.c();
                let c = self.cells.get_at(self.cursor);
                c.letter = Some(x);
                c.style = self.style;
                // eprintln!(
                //     "@ {:?} (mx {w}) c={}",
                //     self.cursor,
                //     c.letter.unwrap()
                // );
                self.cursor.0 += 1;
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
                params: [x],
                end: b'J',
                ..
            }) => {
                for row in match x.value_or(0) {
                    0 => self.cursor.1 + 1..self.cells.r(),
                    1 => 1..self.cursor.1,
                    2 => 1..self.cells.r(),
                    3 => 0..0,
                    _ => unreachable!(),
                } {
                    self.cells.row(row).fill(default());
                }
            }
            Control(ControlFunction {
                start: b'[',
                params: [Default],
                end: b'K',
                ..
            }) => {
                self.cells.past(self.cursor).fill(default());
            }
            Control(ControlFunction {
                start: b'[',
                params: [x],
                end: b'M',
                ..
            }) => {
                let x = x.value_or(1);
                self.cells.grow(x as _);
            }
            Control(ControlFunction {
                start: b'[',
                params: [x],
                end: b'L',
                ..
            }) => {
                let x = x.value_or(1);
                self.cells.insert_lines(x as _, self.cursor.1);
            }
            Control(ControlFunction {
                start: b'[',
                params: [Value(6)],
                end: b'n',
                ..
            }) => {
                super::write(
                    pty,
                    format!("\x1b[{};{}R", self.cursor.1, self.cursor.0)
                        .as_bytes(),
                )
                .unwrap();
            }
            Control(ControlFunction {
                start: b'[',
                params: [p],
                end: b'@',
                ..
            }) => {
                let count = p.value_or(1);
                self.cells.insert_chars(count, self.cursor);
            }
            Control(ControlFunction {
                start: b'[',
                params: [p],
                end: b'P',
                ..
            }) => {
                let count = p.value_or(1);
                self.cells.delete_chars(count, self.cursor);
            }
            Control(ControlFunction {
                start: b'[',
                params: [x],
                end: b'X',
                ..
            }) => {
                let x = x.value_or(1);
                for cell in
                    self.cells.row(self.cursor.1).iter_mut().take(x as _)
                {
                    *cell = default();
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
                params: [y, x],
                end: b'f',
                ..
            }) => {
                self.cursor = (x.value_or(1) as _, y.value_or(1));
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

use crate::colors;
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
