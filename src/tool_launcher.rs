use anyhow::{Context, Result};
use std::process::{Command, Stdio};
use std::collections::HashMap;

pub struct ToolConfig {
    pub name: &'static str,
    pub command: &'static str,
    pub description: &'static str,
    pub requires_shell: bool,
}

/// A dynamic tool configuration for arbitrary commands
pub struct DynamicTool {
    pub name: String,
    pub command: String,
}

static CLAUDE_TOOL: ToolConfig = ToolConfig {
    name: "claude",
    command: "claude",
    description: "Claude Code CLI",
    requires_shell: false,
};

static OPENCLAW_TOOL: ToolConfig = ToolConfig {
    name: "openclaw",
    command: "openclaw",
    description: "OpenClaw CLI",
    requires_shell: false,
};

pub fn get_tool(name: &str) -> Option<&'static ToolConfig> {
    match name.to_lowercase().as_str() {
        "claude" => Some(&CLAUDE_TOOL),
        "openclaw" => Some(&OPENCLAW_TOOL),
        _ => None,
    }
}

#[allow(dead_code)]
pub fn list_tools() -> Vec<&'static ToolConfig> {
    vec![
        &CLAUDE_TOOL,
        &OPENCLAW_TOOL,
    ]
}

/// Launch a predefined tool with proxy configured
pub fn launch_tool(
    tool: &ToolConfig,
    args: &[String],
    proxy_port: u16,
    ca_cert_path: &std::path::Path,
) -> Result<()> {
    launch_tool_impl(tool.name, tool.command, tool.requires_shell, args, proxy_port, ca_cert_path)
}

/// Launch an arbitrary command with proxy configured
pub fn launch_dynamic_tool(
    tool: &DynamicTool,
    args: &[String],
    proxy_port: u16,
    ca_cert_path: &std::path::Path,
) -> Result<()> {
    launch_tool_impl(&tool.name, &tool.command, false, args, proxy_port, ca_cert_path)
}

/// Internal implementation for launching any tool/command
fn launch_tool_impl(
    name: &str,
    command: &str,
    requires_shell: bool,
    args: &[String],
    proxy_port: u16,
    ca_cert_path: &std::path::Path,
) -> Result<()> {
    let mut env_vars = HashMap::new();
    
    // Set proxy environment variables
    let proxy_url = format!("http://127.0.0.1:{}", proxy_port);
    env_vars.insert("HTTP_PROXY", proxy_url.clone());
    env_vars.insert("HTTPS_PROXY", proxy_url.clone());
    env_vars.insert("http_proxy", proxy_url.clone());
    env_vars.insert("https_proxy", proxy_url);
    
    // Set CA certificate environment variables
    let ca_path_str = ca_cert_path.to_string_lossy();
    env_vars.insert("NODE_EXTRA_CA_CERTS", ca_path_str.to_string());
    env_vars.insert("SSL_CERT_FILE", ca_path_str.to_string());
    env_vars.insert("REQUESTS_CA_BUNDLE", ca_path_str.to_string());
    
    // Prepare command
    let mut cmd = if requires_shell {
        let mut shell_cmd = if cfg!(target_os = "windows") {
            let mut c = Command::new("cmd");
            c.arg("/C");
            c
        } else {
            let mut c = Command::new("sh");
            c.arg("-c");
            c
        };
        
        let full_command = if args.is_empty() {
            command.to_string()
        } else {
            format!("{} {}", command, args.join(" "))
        };
        shell_cmd.arg(full_command);
        shell_cmd
    } else {
        let mut c = Command::new(command);
        c.args(args);
        c
    };
    
    // Apply environment variables
    for (key, value) in &env_vars {
        cmd.env(key, value);
    }
    
    // Inherit stdio so the tool runs interactively
    cmd.stdin(Stdio::inherit())
       .stdout(Stdio::inherit())
       .stderr(Stdio::inherit());
    
    println!("🚀 Launching {} with fakekey proxy...", name);
    println!("   Proxy: http://127.0.0.1:{}", proxy_port);
    println!("   CA cert: {}", ca_cert_path.display());
    println!();
    
    // Execute and wait for completion
    let status = cmd.status()
        .with_context(|| format!("Failed to launch {}", name))?;
    
    if !status.success() {
        let code = status.code().unwrap_or(-1);
        anyhow::bail!("{} exited with code {}", name, code);
    }
    
    Ok(())
}
