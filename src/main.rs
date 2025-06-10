#![feature(
    generic_assert,
    deadline_api,
    deref_patterns,
    generic_const_exprs,
    impl_trait_in_bindings,
    if_let_guard,
    import_trait_associated_functions
)]
use std::fs::File;
use std::io::Write;
use std::iter::successors;
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd};
use std::process::{Command, exit};
use std::sync::mpsc;
use std::thread::sleep;
use std::time::{Duration, Instant};

pub mod colors;

use anyhow::Result;
use ctlfun::TerminalInputParser;
use fimg::Image;
use minifb::{InputCallback, Key, WindowOptions};
use nix::pty::{ForkptyResult, forkpty};
use nix::sys::wait::waitpid;
use nix::unistd::Pid;
use render::FONT;
use term::*;
mod render;
mod term;
use libc::{
    F_GETFL, F_SETFL, O_NONBLOCK, TIOCSWINSZ, fcntl, ioctl, winsize,
};
fn spawn(shell: &str) -> Result<(OwnedFd, Pid)> {
    let x = unsafe { forkpty(None, None)? };
    match x {
        ForkptyResult::Child => {
            let sh =
                Command::new(shell).env("TERM", "xterm").spawn()?.wait();
            exit(0);
        }
        ForkptyResult::Parent { child, master } => {
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
            Ok((master, child))
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

    let (pty, pid) = spawn("bash")?;
    let pty1 = pty.try_clone()?;
    let pty2 = pty.try_clone()?;

    std::thread::spawn(move || {
        use Key::*;
        let mut shifting = false;
        let mut ctrl = false;
        while let Ok((k, s)) = krx.recv() {
            if !s {
                if k == LeftShift || k == RightShift {
                    shifting = false;
                }
                if k == LeftCtrl || k == RightCtrl {
                    ctrl = false;
                }
                continue;
            }

            let x = match k {
                LeftSuper | RightSuper => continue,
                LeftShift | RightShift => {
                    shifting = true;
                    continue;
                }
                LeftCtrl | RightCtrl => {
                    ctrl = true;
                    continue;
                }
                Enter => &b"\n"[..],
                Escape => b"\x1b",
                Up => b"\x1b[A",
                Down => b"\x1b[B",
                Right => b"\x1b[C",
                Left => b"\x1b[D",
                NumPad7 | Home => b"\x1b[H",
                NumPad1 | End => b"\x1b[F",
                Apostrophe if shifting => b"\"",
                Apostrophe => b"'",
                Space => b" ",
                Period => b".",
                Slash if shifting => b"?",
                Slash => b"/",
                Backslash => b"\\",
                Backspace => b"",
                Equal if shifting => b"+",
                Equal => b"=",
                Tab => b"\t",
                Minus if shifting => b"_",
                Minus => b"-",
                LeftBracket if shifting => b"{",
                LeftBracket => b"[",
                RightBracket => b"]",
                Semicolon if shifting => b":",
                Semicolon => b";",
                Comma => b",",

                Key0 | Key1 | Key2 | Key3 | Key4 | Key5 | Key6 | Key7
                | Key8 | Key9
                    if shifting =>
                {
                    &[b")!@#$%^&*("[k as usize]]
                }

                Key0 | Key1 | Key2 | Key3 | Key4 | Key5 | Key6 | Key7
                | Key8 | Key9 => &[k as u8 + b'0'],
                _ if ctrl => &[k as u8 - 9],
                _ if shifting => &[k as u8 - 10 + b'A'],
                _ => &[k as u8 - 10 + b'a'],
            };
            write(pty1.as_fd(), x).unwrap();
        }
    });

    // output
    let (ttx, trx) = mpsc::channel();

    std::thread::spawn(move || {
        loop {
            let x = successors(read(pty2.as_fd()), |_| read(pty2.as_fd()))
                .flatten()
                .collect::<Vec<u8>>();
            if !x.is_empty() {
                // println!("recv");
                ttx.send(x).unwrap();
            }
            sleep(Duration::from_millis(10))
        }
    });
    std::thread::spawn(move || {
        loop {
            match waitpid(pid, None).unwrap() {
                nix::sys::wait::WaitStatus::Exited(..) => exit(0),
                _ => (),
            }
        }
    });

    sleep(Duration::from_millis(100));
    w.update();
    let ppem = 20.0;
    let (fw, fh) = render::dims(&FONT, ppem);
    let cols = (w.get_size().0 as f32 / fw).floor() as u16 - 1;
    let rows = (w.get_size().1 as f32 / fh).floor() as u16 - 1;
    dbg!(rows, cols);
    let mut t = Terminal::new((cols, rows), false);
    unsafe {
        let x = winsize {
            ws_row: rows,
            ws_col: cols,
            ws_xpixel: w.get_size().0 as _,
            ws_ypixel: w.get_size().1 as _,
        };
        assert!(ioctl(pty.as_raw_fd(), TIOCSWINSZ, &raw const x) == 0);
    };
    let cj =
        swash::FontRef::from_index(&include_bytes!("../cjk.ttc")[..], 0)
            .unwrap();
    let m = cj.metrics(&[]);
    dbg!(m);

    let mut f = File::create("x").unwrap();
    loop {
        while let Ok(x) = trx.recv_timeout(Duration::from_millis(16)) {
            f.write_all(&x)?;
            for char in x {
                t.rx(char);
            }
        }
        let i = render::render(&mut t, w.get_size(), ppem);
        let x = Image::<Box<[u32]>, 1>::from(i.as_ref());
        w.update_with_buffer(x.buffer(), w.get_size().0, w.get_size().1)?;
    }

    Ok(())
}
#[test]

fn tpaxrse() {
    println!("-------------------");
    let mut x = TerminalInputParser::new();
    for c in "\x1b[?1049h".as_bytes() {
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
