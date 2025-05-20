#![feature(deadline_api, deref_patterns)]
use std::fs::File;
use std::io::Write;
use std::iter::successors;
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd};
use std::process::{Command, exit};
use std::sync::mpsc;
use std::thread::sleep;
use std::time::Duration;

use anyhow::Result;
use ctlfun::TerminalInputParser;
use fimg::Image;
use minifb::{InputCallback, Key, WindowOptions};
use nix::pty::{ForkptyResult, forkpty};
use render::FONT;
use term::*;
mod render;
mod term;
fn spawn(shell: &str) -> Result<OwnedFd> {
    let x = unsafe { forkpty(None, None)? };
    match x {
        ForkptyResult::Child => {
            let sh = Command::new(shell).spawn()?.wait();
            // std::thread::sleep(Duration::from_millis(5000));
            // exit(0);

            exit(0);
        }
        ForkptyResult::Parent { child, master } => {
            use libc::{F_GETFL, F_SETFL, O_NONBLOCK, fcntl};
            unsafe {
                assert_eq!(
                    fcntl(
                        master.as_raw_fd(),
                        F_SETFL,
                        fcntl(master.as_raw_fd(), F_GETFL, 0) | O_NONBLOCK,
                    ),
                    0
                )
            };
            Ok(master)
        }
    }
}

fn read(fd: BorrowedFd) -> Option<Vec<u8>> {
    let mut x = [0; 1 << 16];
    let n = nix::unistd::read(fd, &mut x).ok()?;
    Some(x[..n].to_vec())
}
fn write(fd: BorrowedFd, x: &[u8]) -> Result<()> {
    let n = nix::unistd::write(fd, x)?;
    anyhow::ensure!(n == x.len());
    Ok(())
}

struct KeyPress(mpsc::Sender<(Key, bool)>);
impl InputCallback for KeyPress {
    fn add_char(&mut self, _: u32) {}
    fn set_key_state(&mut self, key: Key, state: bool) {
        self.0.send((key, state)).unwrap();
    }
}
enum Event {
    Read(Vec<u8>),
    Write(Key),
}
fn main() -> Result<()> {
    let mut w = minifb::Window::new(
        "pattypan",
        5,
        5,
        WindowOptions {
            borderless: true,
            title: false,
            resize: true,
            ..Default::default()
        },
    )?;

    // input
    let (ktx, krx) = mpsc::channel();

    w.set_input_callback(Box::new(KeyPress(ktx)));
    w.update();

    let pty = spawn("bash")?;
    let pty1 = pty.try_clone()?;

    std::thread::spawn(move || {
        use Key::*;
        let mut shifting = false;
        while let Ok((k, s)) = krx.recv() {
            if s == true {
                if k == LeftShift || k == RightShift {
                    shifting = true;
                    continue;
                }
                let x = match k {
                    Enter => b"\n",
                    Space => b" ",
                    Period => b".",
                    Slash => b"/",
                    Backslash => b"\\",
                    Backspace => b"",
                    LeftBracket => b"[",
                    RightBracket => b"]",
                    Semicolon if shifting => b":",
                    Semicolon => b";",
                    Comma => b",",

                    Key0 | Key1 | Key2 | Key3 | Key4 | Key5 | Key6
                    | Key7 | Key8 | Key9 => &[k as u8 + b'0'],

                    _ => &[k as u8 - 10 + b'a'],
                };
                write(pty1.as_fd(), x).unwrap();
            } else {
                if k == LeftShift || k == RightShift {
                    shifting = false;
                }
            }
        }
    });

    // output
    let (ttx, trx) = mpsc::channel();

    std::thread::spawn(move || {
        loop {
            let x = successors(read(pty.as_fd()), |_| read(pty.as_fd()))
                .flatten()
                .collect::<Vec<u8>>();
            if !x.is_empty() {
                // println!("recv");
                ttx.send(x).unwrap();
            }
            sleep(Duration::from_millis(10))
        }
    });

    sleep(Duration::from_millis(100));
    w.update();
    let ppem = 18.0;
    let (fw, fh) = render::dims(&FONT, ppem);
    let cols = (w.get_size().0 as f32 / fw).floor() as u16;
    let rows = (w.get_size().1 as f32 / fh).floor() as u16;
    dbg!(rows, cols);
    let mut t = Terminal {
        cursor: (1, 1),
        size: (cols, rows),
        scrollback: Scrollback::default(),
        cells: vec![
            Cell {
                bg: [0; 3],
                color: [0; 3],
                style: 0,
                letter: None,
            };
            cols as usize * rows as usize
        ],
        p: Default::default(),
        mode: Mode::Normal,
    };
    let mut f = File::create("x").unwrap();
    loop {
        while let Ok(x) = trx.recv_timeout(Duration::from_millis(16)) {
            f.write_all(&x)?;
            for char in x {
                t.rx(char);
            }
        }
        let i = render::render(&t, w.get_size(), ppem);
        let x = Image::<Box<[u32]>, 1>::from(i.as_ref());
        w.update_with_buffer(x.buffer(), w.get_size().0, w.get_size().1)?;
    }

    Ok(())
}
#[test]

fn tparse() {
    println!("-------------------");
    let mut x = TerminalInputParser::new();
    for c in "\x1b[32m greninator \x1b[0m".as_bytes() {
        use ctlfun::TerminalInput::*;
        match x.parse_byte(*c) {
            Char(x) => {
                print!("{x}");
            }

            Control(x) => println!("{x:?}"),
            _ => (),
        }
    }
    panic!();
}
