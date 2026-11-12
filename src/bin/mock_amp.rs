use hegeldt::mock::MockServer;
use hegeldt::protocol::AMP_PORT;

fn main() -> anyhow::Result<()> {
    let port = match std::env::args().nth(1) {
        Some(p) => p.parse()?,
        None => AMP_PORT,
    };
    let server = MockServer::start(port, true)?;
    println!("[mock] listening on {}", server.addr());
    loop {
        std::thread::park();
    }
}
