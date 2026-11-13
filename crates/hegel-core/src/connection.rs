//! Non-blocking connection to an amplifier. The caller must supply the host;
//! there is no default host anywhere in this crate.

use std::io::{Read, Write};
use std::net::{Shutdown, TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender, unbounded};

use crate::protocol::{Command, Param, Status, build, parse_status};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(1);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AmpEvent {
    Connected,
    Status(Status),
    Disconnected { reason: String },
}

pub struct AmpConnection {
    tx: Sender<String>,
    closing: Arc<AtomicBool>,
    stream: Arc<Mutex<Option<TcpStream>>>,
}

impl AmpConnection {
    pub fn connect(host: String, port: u16) -> (AmpConnection, UnboundedReceiver<AmpEvent>) {
        let (ev_tx, ev_rx) = unbounded();
        let (tx, rx) = channel::<String>();
        let closing = Arc::new(AtomicBool::new(false));
        let stream = Arc::new(Mutex::new(None));
        let conn = AmpConnection {
            tx: tx.clone(),
            closing: closing.clone(),
            stream: stream.clone(),
        };
        thread::spawn(move || run(host, port, tx, rx, ev_tx, closing, stream));
        (conn, ev_rx)
    }

    pub fn send(&self, cmd: Command, param: Param) {
        let _ = self.tx.send(build(cmd, param));
    }

    pub fn close(&self) {
        self.closing.store(true, Ordering::SeqCst);
        if let Some(s) = self.stream.lock().unwrap().take() {
            let _ = s.shutdown(Shutdown::Both);
        }
    }
}

impl Drop for AmpConnection {
    fn drop(&mut self) {
        self.close();
    }
}

fn open(host: &str, port: u16) -> Result<TcpStream, String> {
    let addrs = (host, port)
        .to_socket_addrs()
        .map_err(|e| format!("cannot resolve {host}: {e}"))?;
    let mut last = format!("no address for {host}");
    for addr in addrs {
        match TcpStream::connect_timeout(&addr, CONNECT_TIMEOUT) {
            Ok(s) => return Ok(s),
            Err(e) => last = format!("could not connect to {addr}: {e}"),
        }
    }
    Err(last)
}

fn run(
    host: String,
    port: u16,
    tx: Sender<String>,
    rx: std::sync::mpsc::Receiver<String>,
    events: UnboundedSender<AmpEvent>,
    closing: Arc<AtomicBool>,
    shared: Arc<Mutex<Option<TcpStream>>>,
) {
    let disconnect = |reason: String| {
        if !closing.load(Ordering::SeqCst) {
            let _ = events.unbounded_send(AmpEvent::Disconnected { reason });
        }
    };
    let mut stream = match open(&host, port) {
        Ok(s) => s,
        Err(reason) => return disconnect(reason),
    };
    let _ = stream.set_nodelay(true);
    let writer = match stream.try_clone() {
        Ok(w) => w,
        Err(e) => return disconnect(format!("socket error: {e}")),
    };
    {
        let mut guard = shared.lock().unwrap();
        if closing.load(Ordering::SeqCst) {
            return;
        }
        match stream.try_clone() {
            Ok(c) => *guard = Some(c),
            Err(e) => return disconnect(format!("socket error: {e}")),
        }
    }
    // Writer thread; ends when all senders are dropped or a write fails.
    thread::spawn(move || {
        let mut writer = writer;
        for msg in rx {
            if writer.write_all(msg.as_bytes()).is_err() {
                break;
            }
        }
    });

    let _ = events.unbounded_send(AmpEvent::Connected);
    let _ = tx.send(build(Command::Power, Param::Query));

    let (mut know_mute, mut know_volume, mut know_input) = (false, false, false);
    let mut buf = [0u8; 512];
    let mut line: Vec<u8> = Vec::new();
    loop {
        let n = match stream.read(&mut buf) {
            Ok(0) => return disconnect("connection closed by amplifier".into()),
            Ok(n) => n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return disconnect(format!("connection lost: {e}")),
        };
        for &b in &buf[..n] {
            if b != b'\r' && b != b'\n' {
                if line.len() < 4096 {
                    line.push(b);
                }
                continue;
            }
            if line.is_empty() {
                continue;
            }
            let text = String::from_utf8_lossy(&line).into_owned();
            line.clear();
            let Some(status) = parse_status(&text) else {
                continue;
            };
            let ask = |c: Command| {
                let _ = tx.send(build(c, Param::Query));
            };
            match status.cmd {
                Command::Power => {
                    if status.value == 0 {
                        (know_mute, know_volume, know_input) = (false, false, false);
                    } else if !know_mute {
                        ask(Command::Mute);
                    }
                }
                Command::Mute => {
                    know_mute = true;
                    if !know_volume {
                        ask(Command::Volume);
                    }
                }
                Command::Volume => {
                    know_volume = true;
                    if !know_input {
                        ask(Command::Input);
                    }
                }
                Command::Input => know_input = true,
                Command::Reset => continue,
            }
            if events.unbounded_send(AmpEvent::Status(status)).is_err() {
                return;
            }
        }
        if closing.load(Ordering::SeqCst) {
            return;
        }
    }
}
