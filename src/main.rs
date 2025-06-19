#![feature(
    debug_closure_helpers,
    const_trait_impl,
    generic_assert,
    deadline_api,
    deref_patterns,
    generic_const_exprs,
    guard_patterns,
    impl_trait_in_bindings,
    if_let_guard,
    import_trait_associated_functions
)]
#![allow(incomplete_features)]
use std::fs::File;
use std::io::Write;
use std::iter::successors;
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd};
use std::process::{Command, exit};
use std::sync::mpsc;
use std::thread::sleep;
use std::time::{Duration, Instant};

pub mod colors;
mod keyboard;

use anyhow::Result;
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
            _ = Command::new(shell)
                .env("TERM", "pattypan")
                .spawn()?
                .wait();
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
        let mut b = keyboard::Board::new();
        while let Ok((k, s)) = krx.recv() {
            let x = b.rx(k, s);
            write(pty1.as_fd(), &x).unwrap();
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
            sleep(Duration::from_millis(10));
        }
    });
    std::thread::spawn(move || {
        loop {
            if let nix::sys::wait::WaitStatus::Exited(..) =
                waitpid(pid, None).unwrap()
            {
                exit(0)
            }
        }
    });

    while w.get_size().0 < 20 || w.get_size().0 > 5000 {
        sleep(Duration::from_millis(10));
        w.update();
    }
    let ppem = 20.0;
    let (fw, fh) = render::dims(&FONT, ppem);
    let cols = (w.get_size().0 as f32 / fw).floor() as u16 - 1;
    let rows = (w.get_size().1 as f32 / fh).floor() as u16 - 1;
    println!("{}x{}", rows, cols);
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
        t.scroll(w.get_scroll_wheel().unwrap_or_default().1);
        while let Ok(x) = trx.recv_timeout(Duration::from_millis(16)) {
            f.write_all(&x)?;
            for char in x {
                t.rx(char, pty.as_fd());
            }
        }
        // dbg!(t.cells.get_at((1, 1)));
        let i = render::render(&mut t, w.get_size(), ppem);
        let x = Image::<Box<[u32]>, 1>::from(i.as_ref());
        w.update_with_buffer(x.buffer(), w.get_size().0, w.get_size().1)?;
    }
}
#[test]

fn tpaxrse() {
    println!("-------------------");
    use ctlfun::TerminalInputParser;
    let mut x = TerminalInputParser::new();
    for c in "\x1b[6n".as_bytes() {
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
