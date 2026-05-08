use anyhow::Result;
use std::process::Command;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct TcpConnection {
    pub peer_addr: String,
    pub bytes_sent: u64,
    pub bytes_received: u64,
}

/// Collect active TCP connections on port 3260 by parsing `ss -tnpi sport 3260`.
pub fn collect_tcp_connections() -> Result<Vec<TcpConnection>> {
    let output = Command::new("ss")
        .args(["-tnpi", "sport", "3260"])
        .output()?;

    let text = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = text.lines().collect();
    let mut connections = Vec::new();

    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        // Look for ESTAB lines
        if line.starts_with("ESTAB") {
            let fields: Vec<&str> = line.split_whitespace().collect();
            // fields: State Recv-Q Send-Q Local Peer
            let peer = fields.get(4).copied().unwrap_or("").to_string();
            let peer_addr = strip_port(&peer);

            let mut bytes_sent = 0u64;
            let mut bytes_received = 0u64;

            // Next line is tab-indented and contains socket stats
            if let Some(stats_line) = lines.get(i + 1) {
                if stats_line.starts_with('\t') || stats_line.starts_with(' ') {
                    for token in stats_line.split_whitespace() {
                        if let Some(v) = token.strip_prefix("bytes_sent:") {
                            bytes_sent = v.parse().unwrap_or(0);
                        } else if let Some(v) = token.strip_prefix("bytes_received:") {
                            bytes_received = v.parse().unwrap_or(0);
                        }
                    }
                    i += 1;
                }
            }

            connections.push(TcpConnection {
                peer_addr,
                bytes_sent,
                bytes_received,
            });
        }
        i += 1;
    }

    Ok(connections)
}

/// Strip port from an address like `[2001:db8::1]:12345` → `2001:db8::1`
/// or `192.168.1.1:12345` → `192.168.1.1`.
fn strip_port(addr: &str) -> String {
    if addr.starts_with('[') {
        // IPv6 with brackets: [addr]:port
        if let Some(end) = addr.find(']') {
            return addr[1..end].to_string();
        }
    }
    // IPv4: addr:port — strip last colon-separated segment
    if let Some(colon) = addr.rfind(':') {
        return addr[..colon].to_string();
    }
    addr.to_string()
}
