use std::process::Command;

use anyhow::{Result, bail};

/// These operations have no public Win32 API; Windows only exposes them
/// through `netsh` / `ipconfig`, so we invoke those tools directly.
fn run(program: &str, args: &[&str]) -> Result<Vec<String>> {
    let output = Command::new(program).args(args).output()?;
    let mut lines = Vec::new();
    for chunk in [output.stdout, output.stderr] {
        let text = String::from_utf8_lossy(&chunk);
        for line in text.lines() {
            let line = line.trim();
            if !line.is_empty() {
                lines.push(line.to_string());
            }
        }
    }
    if !output.status.success() {
        bail!(
            "{} {} exited with {}",
            program,
            args.join(" "),
            output.status
        );
    }
    Ok(lines)
}

pub fn winsock_reset() -> Result<Vec<String>> {
    run("netsh", &["winsock", "reset"])
}

pub fn ip_reset() -> Result<Vec<String>> {
    run("netsh", &["int", "ip", "reset"])
}

pub fn register_dns() -> Result<Vec<String>> {
    run("ipconfig", &["/registerdns"])
}

pub fn flush_dns_fallback() -> Result<Vec<String>> {
    run("ipconfig", &["/flushdns"])
}
