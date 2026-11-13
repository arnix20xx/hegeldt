//! Runs SSDP discovery once and prints Hegel devices. Sends multicast UDP on the LAN.
use std::time::Duration;

fn main() {
    match hegel_core::discovery::search(Duration::from_secs(5)) {
        Ok(devs) if devs.is_empty() => println!("no Hegel devices found"),
        Ok(devs) => devs.iter().for_each(|d| println!("{d:?}")),
        Err(e) => eprintln!("discovery failed: {e}"),
    }
}
