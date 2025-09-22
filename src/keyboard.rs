use atools::prelude::*;
use minifb::Key;
#[derive(Clone, Copy)]
pub struct Modifiers {
    shift: bool,
    alt: bool,
    ctrl: bool,
}

impl Modifiers {
    #[implicit_fn::implicit_fn]
    fn x(self) -> u8 {
        [self.shift, self.alt, self.ctrl]
            .iter()
            .zip(0..)
            .filter(*_.0)
            .fold(0u8, |acc, x| acc | 1 << x.1)
    }
}

pub struct Board {
    mods: Modifiers,
}
impl Board {
    pub fn new() -> Self {
        Self { mods: Modifiers { alt: false, shift: false, ctrl: false } }
    }

    pub fn rx(&mut self, k: Key, s: bool) -> Vec<u8> {
        use Key::*;
        let Self { mods: Modifiers { shift, ctrl, alt } } = self;
        if !s {
            if k == LeftAlt || k == RightAlt {
                *alt = false;
            }
            if k == LeftShift || k == RightShift {
                *shift = false;
            }
            if k == LeftCtrl || k == RightCtrl {
                *ctrl = false;
            }
            return vec![];
        }

        let l = match k as u8 {
            k @ 10..=34 if *shift => Some(k - 10 + b'A'),
            k @ 10..=34 => Some(k - 10 + b'a'),
            _ => None,
        }
        .map(char::from);
        let mut v = vec![];
        match k {
            LeftSuper | RightSuper => {}
            LeftAlt | RightAlt => {
                *alt = true;
            }
            LeftShift | RightShift => {
                *shift = true;
            }
            LeftCtrl | RightCtrl => {
                *ctrl = true;
            }
            k if let Some(l) = l
                && *ctrl =>
            {
                let k = l.to_ascii_uppercase() as u8 & 0x1f;
                if *alt {
                    v.push(0x1b);
                }
                v.push(k);
            }
            _ if let Some(l) = l
                && *alt =>
            {
                v.extend([0x1b, l as u8]);
            }
            Enter => v.extend(b"\n"),
            Delete => v.extend(b"\x1b[3~"),
            Insert => v.extend(b"\x1b[2~"),
            Escape => v.extend(b"\x1b"),
            Up | Down | Left | Right | Home | End | NumPad7 | NumPad1 => {
                let c = match k {
                    Up => b'A',
                    Down => b'B',
                    Right => b'C',
                    Left => b'D',
                    Home | NumPad7 => b'H',
                    End | NumPad1 => b'F',
                    _ => unreachable!(),
                };
                if *shift | *alt | *ctrl {
                    v.extend(
                        (*b"\x1b[1;")
                            .join(1 + self.mods.x() + b'0')
                            .join(c),
                    );
                } else {
                    v.extend((*b"\x1b[").join(c));
                }
            }
            Apostrophe if *shift => v.extend(b"\""),
            Apostrophe => v.extend(b"'"),
            Space => v.extend(b" "),
            Period if *shift => v.extend(b">"),
            Period => v.extend(b"."),
            Slash if *shift => v.extend(b"?"),
            Slash => v.extend(b"/"),
            Backslash if *shift => v.extend(b"|"),
            Backslash => v.extend(b"\\"),
            Backspace => v.extend(b""),
            Equal if *shift => v.extend(b"+"),
            Equal => v.extend(b"="),
            Tab => v.extend(b"\t"),
            Minus if *shift => v.extend(b"_"),
            Minus => v.extend(b"-"),
            LeftBracket if *shift => v.extend(b"{"),
            LeftBracket => v.extend(b"["),
            RightBracket => v.extend(b"]"),
            Semicolon if *shift => v.extend(b":"),
            Semicolon => v.extend(b";"),
            Comma if *shift => v.extend(b"<"),
            Comma => v.extend(b","),
            Backquote if *shift => v.extend(b"~"),
            Backquote => v.extend(b"`"),

            Key0 | Key1 | Key2 | Key3 | Key4 | Key5 | Key6 | Key7
            | Key8 | Key9
                if *shift =>
            {
                v.push(b")!@#$%^&*("[k as usize]);
            }

            Key0 | Key1 | Key2 | Key3 | Key4 | Key5 | Key6 | Key7
            | Key8 | Key9 => v.push(k as u8 + b'0'),

            _ if *ctrl => v.push(k as u8 - 9),
            _ if *shift => v.push(k as u8 - 10 + b'A'),
            _ => v.push(k as u8 - 10 + b'a'),
        };
        v
    }
}
