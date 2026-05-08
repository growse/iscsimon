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
        if line.starts_with("ESTAB") {
            let fields: Vec<&str> = line.split_whitespace().collect();
            let peer = fields.get(4).copied().unwrap_or("").to_string();
            let peer_addr = strip_port(&peer);

            let mut bytes_sent = 0u64;
            let mut bytes_received = 0u64;

            if let Some(stats_line) = lines.get(i + 1)
                && (stats_line.starts_with('\t') || stats_line.starts_with(' '))
            {
                for token in stats_line.split_whitespace() {
                    if let Some(v) = token.strip_prefix("bytes_sent:") {
                        bytes_sent = v.parse().unwrap_or(0);
                    } else if let Some(v) = token.strip_prefix("bytes_received:") {
                        bytes_received = v.parse().unwrap_or(0);
                    }
                }
                i += 1;
            }

            connections.push(TcpConnection { peer_addr, bytes_sent, bytes_received });
        }
        i += 1;
    }

    Ok(connections)
}

/// Reverse-DNS lookup an IP address using `getent hosts`. Returns the hostname or None.
pub fn resolve_hostname(ip: &str) -> Option<String> {
    let output = Command::new("getent").args(["hosts", ip]).output().ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    // Output format: "IP  hostname  [aliases...]"
    text.split_whitespace().nth(1).map(str::to_string)
}

/// Strip port from an address: `[2001:db8::1]:12345` → `2001:db8::1`, `1.2.3.4:5678` → `1.2.3.4`.
fn strip_port(addr: &str) -> String {
    if addr.starts_with('[')
        && let Some(end) = addr.find(']')
    {
        return addr[1..end].to_string();
    }
    if let Some(colon) = addr.rfind(':') {
        return addr[..colon].to_string();
    }
    addr.to_string()
}

