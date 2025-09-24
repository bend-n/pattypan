#![feature(
    super_let,
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
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd};
use std::process::{Command, exit};
use std::sync::{LazyLock, mpsc};
use std::thread::sleep;
use std::time::{Duration, Instant};

pub mod colors;
mod keyboard;

use anyhow::Result;
use dsb::F;
use fimg::Image;
use minifb::{InputCallback, Key, WindowOptions};
use nix::pty::{ForkptyResult, forkpty};
use nix::sys::wait::waitpid;
use nix::unistd::Pid;
use swash::{FontRef, Instance};
use term::*;
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
    let init = Instant::now();
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
            if let Some(x) = read(pty2.as_fd()) {
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
        sleep(Duration::from_millis(1));
        w.update();
    }
    println!("{:?}", init.elapsed());
    let ppem = 20.0;
    let mut fonts = dsb::Fonts::new(
        F::FontRef(*FONT, &[(2003265652, 550.0)]),
        F::instance(*FONT, *BFONT),
        F::FontRef(*IFONT, &[(2003265652, 550.0)]),
        F::instance(*IFONT, *BIFONT),
    );

    // let (fw, fh) = dsb::dims(&FONT, ppem);
    let (cols, rows) = dsb::fit(&FONT, ppem, 20.0, w.get_size());
    println!("{}x{}", rows, cols);
    let mut t = Terminal::new((cols as _, rows as _), false);
    t.alternate.as_mut().unwrap().view_o = None;
    let mut i = Image::build(w.get_size().0 as _, w.get_size().1 as _)
        .fill(colors::BACKGROUND);
    unsafe {
        let x = winsize {
            ws_row: rows as _,
            ws_col: cols as _,
            ws_xpixel: w.get_size().0 as _,
            ws_ypixel: w.get_size().1 as _,
        };
        assert!(ioctl(pty.as_raw_fd(), TIOCSWINSZ, &raw const x) == 0);
    };
    let cj =
        swash::FontRef::from_index(&include_bytes!("../cjk.ttc")[..], 0)
            .unwrap();

    // let mut f = File::create("x").unwrap();
    loop {
        while w.get_size().0 < 20
            || w.get_size().0 > 5000
            || w.get_size().1 < 20
            || w.get_size().1 > 5000
        {
            sleep(Duration::from_millis(10));
            w.update();
        }
        let (cols, rows) =
            dsb::fit(&FONT, ppem, 20.0, (w.get_size().0, w.get_size().1));
        let [cols, rows] = [cols, rows].map(|x| x as u16 - 1);
        if (cols + 1, rows + 1) != t.cells.size {
            println!("{}x{}", rows, cols);

            unsafe {
                let x = winsize {
                    ws_row: rows,
                    ws_col: cols,
                    ws_xpixel: w.get_size().0 as _,
                    ws_ypixel: w.get_size().1 as _,
                };
                assert!(
                    ioctl(pty.as_raw_fd(), TIOCSWINSZ, &raw const x) == 0
                );
            };
            t.resize((cols, rows));
            i = Image::build(w.get_size().0 as u32, w.get_size().1 as u32)
                .fill(colors::BACKGROUND);
        }

        t.scroll(w.get_scroll_wheel().unwrap_or_default().1);
        let now = Instant::now();
        while let Ok(x) =
            trx.recv_deadline(now + Duration::from_millis(16))
        {
            // f.write_all(&x)?;
            for char in x {
                t.rx(char, pty.as_fd());
            }
        }
        unsafe {
            let z = (
                cols as usize + 1,
                (t.cells.cells().len() / cols as usize + 1)
                    .min(t.cells.r() as _),
            );
            dsb::render(
                &t.cells.cells[t.view_o.unwrap_or(t.cells.row)
                    * t.cells.c() as usize..],
                z,
                ppem,
                colors::BACKGROUND,
                &mut fonts,
                20.0,
                true,
                i.as_mut(),
            )
        };

        let x = Image::<Box<[u32]>, 1>::from(i.as_ref());
        w.update_with_buffer(x.buffer(), w.get_size().0, w.get_size().1)?;
    }
}
pub static FONT: LazyLock<FontRef<'static>> = LazyLock::new(|| {
    FontRef::from_index(
        &include_bytes!("/home/os/CascadiaCodeNF.ttf")[..],
        0,
    )
    .unwrap()
});
pub static IFONT: LazyLock<FontRef<'static>> = LazyLock::new(|| {
    FontRef::from_index(
        &include_bytes!("/home/os/CascadiaCodeNFItalic.ttf")[..],
        0,
    )
    .unwrap()
});

pub static BIFONT: LazyLock<Instance<'static>> = LazyLock::new(|| {
    IFONT.instances().find_by_name("Bold Italic").unwrap()
});

pub static BFONT: LazyLock<Instance<'static>> =
    LazyLock::new(|| FONT.instances().find_by_name("Bold").unwrap());
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
