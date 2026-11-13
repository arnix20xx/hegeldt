//! SSDP discovery of Hegel amplifiers (port of SsdpDiscovery.kt).
//!
//! `search()` sends multicast UDP on the LAN and fetches device descriptions over HTTP. It is
//! used by the "Search for amplifiers" button in the settings (run on a background thread).

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream, ToSocketAddrs, UdpSocket};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredDevice {
    pub friendly_name: String,
    pub host: String,
    pub model: String,
    pub manufacturer: String,
}

const SSDP_ADDR: &str = "239.255.255.250:1900";
const SEARCH_TARGET: &str = "urn:schemas-upnp-org:device:MediaRenderer:1";

pub fn m_search_request() -> String {
    format!(
        "M-SEARCH * HTTP/1.1\r\nHOST: {SSDP_ADDR}\r\nMAN: \"ssdp:discover\"\r\nMX: 2\r\nST: {SEARCH_TARGET}\r\n\r\n"
    )
}

/// Extracts the LOCATION header from an SSDP response.
pub fn parse_location(response: &str) -> Option<String> {
    response.lines().find_map(|l| {
        let (k, v) = l.split_once(':')?;
        k.trim()
            .eq_ignore_ascii_case("location")
            .then(|| v.trim().to_string())
    })
}

/// Extracts the text of the first `<tag>…</tag>` occurrence (namespace-agnostic, no nesting).
fn xml_text(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let start = xml.find(&open)? + open.len();
    let end = xml[start..].find(&format!("</{tag}>"))? + start;
    Some(xml[start..end].trim().to_string())
}

pub fn parse_description(xml: &str, host: &str) -> DiscoveredDevice {
    DiscoveredDevice {
        friendly_name: xml_text(xml, "friendlyName").unwrap_or_default(),
        host: host.to_string(),
        model: xml_text(xml, "modelName").unwrap_or_default(),
        manufacturer: xml_text(xml, "manufacturer").unwrap_or_default(),
    }
}

/// Splits `http://host:port/path` into (host, port, path).
fn split_url(url: &str) -> Option<(String, u16, String)> {
    let rest = url.strip_prefix("http://")?;
    let (authority, path) = rest
        .split_once('/')
        .map_or((rest, "/".to_string()), |(a, p)| (a, format!("/{p}")));
    let (host, port) = match authority.rsplit_once(':') {
        Some((h, p)) => (h.to_string(), p.parse().ok()?),
        None => (authority.to_string(), 80),
    };
    Some((host, port, path))
}

fn http_get(url: &str) -> Option<String> {
    let (host, port, path) = split_url(url)?;
    let addr = (host.as_str(), port).to_socket_addrs().ok()?.next()?;
    let mut s = TcpStream::connect_timeout(&addr, Duration::from_secs(2)).ok()?;
    s.set_read_timeout(Some(Duration::from_secs(2))).ok()?;
    write!(
        s,
        "GET {path} HTTP/1.0\r\nHost: {host}:{port}\r\nConnection: close\r\n\r\n"
    )
    .ok()?;
    let mut buf = String::new();
    s.read_to_string(&mut buf).ok()?;
    buf.split_once("\r\n\r\n").map(|(_, body)| body.to_string())
}

/// Sends an SSDP M-SEARCH and returns the Hegel devices that answered within `timeout`.
/// Network access: multicast UDP plus HTTP GETs to the responders.
pub fn search(timeout: Duration) -> std::io::Result<Vec<DiscoveredDevice>> {
    // Bind to the local address of the default route so multicast leaves via the right
    // interface (binding to 0.0.0.0 fails with "No route to host" on macOS). Connecting a
    // UDP socket sends no packets; it only selects the route.
    let local_ip = {
        let probe = UdpSocket::bind(SocketAddr::from((Ipv4Addr::UNSPECIFIED, 0)))?;
        probe.connect("8.8.8.8:80")?;
        probe.local_addr()?.ip()
    };
    let sock = UdpSocket::bind(SocketAddr::new(local_ip, 0))?;
    sock.set_read_timeout(Some(Duration::from_millis(300)))?;
    sock.send_to(m_search_request().as_bytes(), SSDP_ADDR)?;

    let mut found: HashMap<IpAddr, DiscoveredDevice> = HashMap::new();
    let mut buf = [0u8; 2048];
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        let Ok((n, from)) = sock.recv_from(&mut buf) else {
            continue;
        };
        if found.contains_key(&from.ip()) {
            continue;
        }
        let resp = String::from_utf8_lossy(&buf[..n]);
        let Some(loc) = parse_location(&resp) else {
            continue;
        };
        if let Some(xml) = http_get(&loc) {
            let dev = parse_description(&xml, &from.ip().to_string());
            if dev.manufacturer.eq_ignore_ascii_case("hegel") {
                found.insert(from.ip(), dev);
            }
        }
    }
    Ok(found.into_values().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_location_case_insensitively() {
        let r = "HTTP/1.1 200 OK\r\nCACHE-CONTROL: max-age=1800\r\nLocation: http://10.0.0.5:49152/desc.xml\r\nST: x\r\n\r\n";
        assert_eq!(
            parse_location(r).as_deref(),
            Some("http://10.0.0.5:49152/desc.xml")
        );
        assert_eq!(parse_location("HTTP/1.1 200 OK\r\n\r\n"), None);
    }

    #[test]
    fn parses_description() {
        let xml = "<root><device><friendlyName>Hegel H390</friendlyName><manufacturer>Hegel</manufacturer><modelName>H390</modelName></device></root>";
        let d = parse_description(xml, "10.0.0.5");
        assert_eq!(d.friendly_name, "Hegel H390");
        assert_eq!(d.manufacturer, "Hegel");
        assert_eq!(d.model, "H390");
        assert_eq!(d.host, "10.0.0.5");
    }

    #[test]
    fn splits_urls() {
        assert_eq!(
            split_url("http://10.0.0.5:49152/desc.xml"),
            Some(("10.0.0.5".into(), 49152, "/desc.xml".into()))
        );
        assert_eq!(
            split_url("http://host/x"),
            Some(("host".into(), 80, "/x".into()))
        );
        assert_eq!(split_url("https://x/"), None);
    }

    #[test]
    fn search_request_targets_media_renderer() {
        let r = m_search_request();
        assert!(r.starts_with("M-SEARCH * HTTP/1.1\r\n"));
        assert!(r.contains(SEARCH_TARGET));
        assert!(r.ends_with("\r\n\r\n"));
    }
}
