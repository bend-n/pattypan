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
            }) if params.is_empty() => {
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
                        _ => {}
                    }
                }
            }
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
#[derive(Clone, Copy, Debug)]
enum StyleAction {
    Reset,
    ModeSet(u16),
    SetFg([u8; 3]),
    SetBg([u8; 3]),
}
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
