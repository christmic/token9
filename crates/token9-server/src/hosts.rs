use std::fs::{self, OpenOptions};
use std::io::Write;

const HOSTS_PATH: &str = "/etc/hosts";

fn entry_line(domain: &str) -> String {
    format!("127.0.0.1\t{domain}\t# token9")
}

/// True if `/etc/hosts` already maps `domain` to 127.0.0.1.
fn has_entry(content: &str, domain: &str) -> bool {
    content.lines().any(|line| {
        let line = line.trim();
        if line.starts_with('#') {
            return false;
        }
        let mut toks = line.split_whitespace();
        matches!(toks.next(), Some("127.0.0.1"))
            && toks.take_while(|t| !t.starts_with('#')).any(|t| t == domain)
    })
}

pub fn status(domain: &str) -> anyhow::Result<()> {
    let content = fs::read_to_string(HOSTS_PATH).unwrap_or_default();
    if has_entry(&content, domain) {
        println!("installed: 127.0.0.1 -> {domain} (in {HOSTS_PATH})");
    } else {
        println!("not installed: no 127.0.0.1 -> {domain} entry in {HOSTS_PATH}");
        println!("run `token9 hosts install` to add it");
    }
    Ok(())
}

/// Print the client endpoint URL plus the (root) commands needed to make the
/// port-less URL work: hosts entry (name -> loopback) + pf redirect (80 -> port).
/// token9 never runs these itself — you review and run them with sudo.
pub fn print_endpoint(domain: &str, port: u16) {
    let content = fs::read_to_string(HOSTS_PATH).unwrap_or_default();
    let hosts_ok = has_entry(&content, domain);

    println!("domain : {domain}");
    println!("port   : {port}");
    println!();
    println!("client base_url:");
    println!("  with port (works now, no sudo): http://{domain}:{port}");
    println!("  port-less (needs pf below)     : http://{domain}");
    println!();
    println!("to enable the port-less URL, run as root (one time):");
    if hosts_ok {
        println!("  # 1) hosts entry already present ✓");
    } else {
        println!("  # 1) name -> loopback");
        println!("  echo '127.0.0.1  {domain}  # token9' | sudo tee -a {HOSTS_PATH}");
    }
    println!("  # 2) redirect :80 -> :{port} (does NOT occupy 80; kernel-level)");
    println!(
        "  echo 'rdr pass on lo0 inet proto tcp from any to 127.0.0.1 port 80 -> 127.0.0.1 port {port}' | sudo pfctl -ef -"
    );
    println!();
    println!("verify: curl http://{domain}/healthz");
    println!("note: pf rule resets on reboot; re-run step 2 after reboot.");
}

pub fn install(domain: &str) -> anyhow::Result<()> {
    let content = fs::read_to_string(HOSTS_PATH).unwrap_or_default();
    if has_entry(&content, domain) {
        println!("already installed: 127.0.0.1 -> {domain}");
        return Ok(());
    }

    let line = entry_line(domain);
    match OpenOptions::new().append(true).open(HOSTS_PATH) {
        Ok(mut f) => {
            writeln!(f, "\n{line}")?;
            println!("installed: {line}");
            println!("you can now use http://{domain}:<port>");
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
            println!("cannot write {HOSTS_PATH} (needs root). run this yourself:");
            println!("  echo '{line}' | sudo tee -a {HOSTS_PATH}");
            Ok(())
        }
        Err(e) => Err(e.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_entry() {
        let c = "127.0.0.1 localhost\n127.0.0.1\ttoken9.test\t# token9\n";
        assert!(has_entry(c, "token9.test"));
        assert!(!has_entry(c, "other.test"));
    }

    #[test]
    fn ignores_commented_lines() {
        let c = "# 127.0.0.1 token9.test\n";
        assert!(!has_entry(c, "token9.test"));
    }

    #[test]
    fn requires_loopback_ip() {
        let c = "10.0.0.1 token9.test\n";
        assert!(!has_entry(c, "token9.test"));
    }
}
