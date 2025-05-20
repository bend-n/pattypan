use ctlfun::TerminalInput::*;
use ctlfun::{ControlFunction, TerminalInputParser};

pub struct Terminal {
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
pub struct Cell {
    pub bg: [u8; 3],
    pub color: [u8; 3],
    pub style: u8,
    pub letter: Option<char>,
}

impl Terminal {
    pub fn rx(&mut self, x: u8) {
        match self.p.parse_byte(x) {
            Continue => {}
            Char(x) => {
                self.cursor.0 += 1;
                self.cells[(self.cursor.1 * self.size.0 + self.cursor.0)
                    as usize]
                    .letter = Some(x);
            }
            Control(ControlFunction { start: b'\r', .. }) => {
                self.cursor.0 = 1;
            }
            Control(ControlFunction { start: b'\n', .. }) => {
                self.cursor.1 += 1;
            }
            Control(x) => {
                println!(
                    "{} {:?} {} {}",
                    x.start as char,
                    x.params
                        .iter()
                        .map(|x| match x {
                            ctlfun::Parameter::Default => 0,
                            ctlfun::Parameter::Value(x) => *x,
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
