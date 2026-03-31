use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};

/// Start the proxy as a background process
pub fn daemonize(pid_file: &PathBuf) -> Result<()> {
    let current_exe = std::env::current_exe()
        .with_context(|| "Failed to get current executable path")?;
    
    // Reconstruct args without --foreground flag
    let args: Vec<String> = std::env::args()
        .skip(1)
        .filter(|arg| arg != "--foreground" && arg != "-f")
        .collect();
    
    let child = Command::new(&current_exe)
        .args(&args)
        .env("FAKEKEY_DAEMON", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .with_context(|| "Failed to spawn background process")?;
    
    let pid = child.id();
    fs::write(pid_file, pid.to_string())
        .with_context(|| format!("Failed to write PID file: {}", pid_file.display()))?;
    
    println!("Background process started with PID: {}", pid);
    std::process::exit(0);
}

pub fn is_daemon_mode() -> bool {
    std::env::var("FAKEKEY_DAEMON").is_ok()
}
