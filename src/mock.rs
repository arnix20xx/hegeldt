//! A fake Hegel amplifier. Listens on loopback only.

use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::protocol::{Command, MAX_VOLUME, Param, build, parse_command};

pub const MOCK_MAX_INPUT: u8 = 10;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MockState {
    pub power: bool,
    pub volume: u8,
    pub input: u8,
    pub mute: bool,
}

impl Default for MockState {
    fn default() -> Self {
        MockState {
            power: true,
            volume: 30,
            input: 1,
            mute: false,
        }
    }
}

impl MockState {
    /// Applies a command and returns the status line to reply with (if any).
    pub fn apply(&mut self, cmd: Command, param: Param) -> Option<String> {
        let value = match cmd {
            Command::Power => {
                match param {
                    Param::Value(v) => self.power = v != 0,
                    Param::Toggle => self.power = !self.power,
                    Param::Up => self.power = true,
                    Param::Down => self.power = false,
                    Param::Query | Param::Stop => {}
                }
                self.power as u8
            }
            Command::Mute => {
                match param {
                    Param::Value(v) => self.mute = v != 0,
                    Param::Toggle => self.mute = !self.mute,
                    Param::Up => self.mute = true,
                    Param::Down => self.mute = false,
                    Param::Query | Param::Stop => {}
                }
                self.mute as u8
            }
            Command::Volume => {
                match param {
                    Param::Value(v) => self.volume = v.min(MAX_VOLUME),
                    Param::Up => self.volume = (self.volume + 1).min(MAX_VOLUME),
                    Param::Down => self.volume = self.volume.saturating_sub(1),
                    Param::Toggle | Param::Query | Param::Stop => {}
                }
                self.volume
            }
            Command::Input => {
                match param {
                    Param::Value(v) => self.input = v.clamp(1, MOCK_MAX_INPUT),
                    Param::Up => self.input = self.input % MOCK_MAX_INPUT + 1,
                    Param::Down => {
                        self.input = if self.input <= 1 {
                            MOCK_MAX_INPUT
                        } else {
                            self.input - 1
                        }
                    }
                    Param::Toggle | Param::Query | Param::Stop => {}
                }
                self.input
            }
            Command::Reset => return None,
        };
        Some(build(cmd, Param::Value(value)))
    }
}

pub struct MockServer {
    addr: SocketAddr,
    state: Arc<Mutex<MockState>>,
}

impl MockServer {
    /// Binds 127.0.0.1:`port` (0 = ephemeral) and serves connections on background threads.
    pub fn start(port: u16, verbose: bool) -> std::io::Result<MockServer> {
        Self::start_with_state(port, verbose, MockState::default())
    }

    pub fn start_with_state(
        port: u16,
        verbose: bool,
        state: MockState,
    ) -> std::io::Result<MockServer> {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, port))?;
        let addr = listener.local_addr()?;
        let state = Arc::new(Mutex::new(state));
        let shared = state.clone();
        thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { continue };
                let state = shared.clone();
                thread::spawn(move || serve_connection(stream, state, verbose));
            }
        });
        Ok(MockServer { addr, state })
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn port(&self) -> u16 {
        self.addr.port()
    }

    pub fn state(&self) -> MockState {
        self.state.lock().unwrap().clone()
    }
}

fn serve_connection(mut stream: TcpStream, state: Arc<Mutex<MockState>>, verbose: bool) {
    let peer = stream.peer_addr().ok();
    if verbose {
        println!("[mock] connection from {peer:?}");
    }
    let mut buf = [0u8; 256];
    let mut line = Vec::new();
    loop {
        let n = match stream.read(&mut buf) {
            Ok(0) | Err(_) => break,
            Ok(n) => n,
        };
        for &b in &buf[..n] {
            if b != b'\r' && b != b'\n' {
                if line.len() < 1024 {
                    line.push(b);
                }
                continue;
            }
            if line.is_empty() {
                continue;
            }
            let text = String::from_utf8_lossy(&line).into_owned();
            line.clear();
            if verbose {
                println!("[mock] recv {text:?}");
            }
            let Some((cmd, param)) = parse_command(&text) else {
                continue;
            };
            let reply = state.lock().unwrap().apply(cmd, param);
            if let Some(reply) = reply {
                if verbose {
                    println!("[mock] send {reply:?}");
                }
                if stream.write_all(reply.as_bytes()).is_err() {
                    return;
                }
            }
        }
    }
    if verbose {
        println!("[mock] disconnected {peer:?}");
    }
}
