use std::time::Duration;

use futures::StreamExt;
use futures::channel::mpsc::UnboundedReceiver;
use hegeldt::connection::{AmpConnection, AmpEvent};
use hegeldt::mock::MockServer;
use hegeldt::protocol::{Command, Param, Status};

fn next(rx: &mut UnboundedReceiver<AmpEvent>) -> AmpEvent {
    futures::executor::block_on(async {
        futures::future::select(
            Box::pin(rx.next()),
            Box::pin(futures_timer(Duration::from_secs(3))),
        )
        .await
    })
    .factor_first()
    .0
    .expect("timed out or closed")
}

async fn futures_timer(d: Duration) -> Option<AmpEvent> {
    let (tx, rx) = futures::channel::oneshot::channel::<()>();
    std::thread::spawn(move || {
        std::thread::sleep(d);
        let _ = tx.send(());
    });
    let _ = rx.await;
    None
}

fn st(cmd: Command, value: u8) -> AmpEvent {
    AmpEvent::Status(Status { cmd, value })
}

#[test]
fn startup_chain_and_round_trips() {
    let server = MockServer::start(0, false).unwrap();
    let (conn, mut rx) = AmpConnection::connect("127.0.0.1".into(), server.port());
    assert_eq!(next(&mut rx), AmpEvent::Connected);
    assert_eq!(next(&mut rx), st(Command::Power, 1));
    assert_eq!(next(&mut rx), st(Command::Mute, 0));
    assert_eq!(next(&mut rx), st(Command::Volume, 30));
    assert_eq!(next(&mut rx), st(Command::Input, 1));

    conn.send(Command::Volume, Param::Up);
    assert_eq!(next(&mut rx), st(Command::Volume, 31));
    conn.send(Command::Mute, Param::Toggle);
    assert_eq!(next(&mut rx), st(Command::Mute, 1));
    conn.send(Command::Input, Param::Value(5));
    assert_eq!(next(&mut rx), st(Command::Input, 5));
    conn.send(Command::Volume, Param::Value(200));
    assert_eq!(next(&mut rx), st(Command::Volume, 99));
    assert_eq!(server.state().input, 5);
    conn.close();
}

#[test]
fn power_cycle_restarts_chain() {
    let server = MockServer::start(0, false).unwrap();
    let (conn, mut rx) = AmpConnection::connect("127.0.0.1".into(), server.port());
    for _ in 0..5 {
        next(&mut rx);
    }
    conn.send(Command::Power, Param::Toggle);
    assert_eq!(next(&mut rx), st(Command::Power, 0));
    conn.send(Command::Power, Param::Toggle);
    assert_eq!(next(&mut rx), st(Command::Power, 1));
    assert_eq!(next(&mut rx), st(Command::Mute, 0));
    assert_eq!(next(&mut rx), st(Command::Volume, 30));
    assert_eq!(next(&mut rx), st(Command::Input, 1));
}

#[test]
fn refused_connection_reports_disconnect() {
    // Grab a free loopback port, then release it so nothing listens there.
    let port = {
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        l.local_addr().unwrap().port()
    };
    let (_conn, mut rx) = AmpConnection::connect("127.0.0.1".into(), port);
    assert!(matches!(next(&mut rx), AmpEvent::Disconnected { .. }));
}
