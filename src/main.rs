#![feature(deadline_api)]
use std::io::Write;
use std::iter::successors;
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd};
use std::process::{Command, exit};
use std::sync::mpsc;
use std::thread::sleep;
use std::time::Duration;

use anyhow::Result;
use minifb::{InputCallback, Key, WindowOptions};
use nix::pty::{ForkptyResult, forkpty};

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

struct KeyPress(mpsc::Sender<Key>);
impl InputCallback for KeyPress {
    fn add_char(&mut self, _: u32) {}
    fn set_key_state(&mut self, key: Key, state: bool) {
        if state {
            self.0.send((key)).unwrap();
        }
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
    let (ktx, krx) = mpsc::channel::<Key>();

    w.set_input_callback(Box::new(KeyPress(ktx)));
    w.update();

    let pty = spawn("fish")?;
    let pty1 = pty.try_clone()?;

    std::thread::spawn(move || {
        while let Ok(k) = krx.recv() {
            let x = match k {
                Key::Enter => b"\n",
                Key::Space => b" ",
                _ => &[k as u8 - 10 + b'a'],
            };
            write(pty1.as_fd(), x).unwrap();
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

    // let x = b"echo -e \"\x1b(0lqqqk\nx   \x1b(Bx\nmqqqj";
    // let x = String::from_utf8_lossy(&x);
    // println!("{}", x);
    let mut s = anstream::StripStream::new(std::io::stdout());
    loop {
        while let Ok(x) = trx.recv_timeout(Duration::from_millis(50)) {
            s.write_all(&x)?;
        }
        s.flush()?;
        w.update();
        sleep(Duration::from_millis(10))
    }

    // println!("-------------------");
    // let mut t = TerminalInputParser::new();
    // for char in x {
    //     use ctlfun::TerminalInput::*;
    //     match t.parse_byte(char) {
    //         Continue => {
    //             print!("-");
    //         }
    //         Char(x) => print!("{x}"),
    //         Control(control_function) => println!("{:?}", control_function),
    //         _ => panic!(),
    //     }
    // }

    Ok(())
}
