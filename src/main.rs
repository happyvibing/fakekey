mod audit;
mod cert;
mod cli;
mod config;
mod daemon;
mod key_handler;
mod proxy;
mod security;
mod templates;
mod tool_launcher;

use anyhow::{Context, Result};
use clap::Parser;
use cli::{CertAction, Cli, Commands};
use config::{generate_unique_fake_key, init_data_dir, AppConfig, ApiKeyConfig, ScanLocation};
use std::net::SocketAddr;
use std::process::Stdio;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");
    
    let cli = Cli::parse();

    // Initialize tracing for non-start commands
    if !matches!(cli.command, Commands::Start { .. } | Commands::Onboard) {
        tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
            )
            .init();
    }

    match cli.command {
        Commands::Init => cmd_init()?,
        Commands::Start { port, daemon } => cmd_start(port, daemon).await?,
        Commands::Add {
            name,
            key,
            template,
            header,
            endpoints,
        } => cmd_add(&name, &key, template.as_deref(), header.as_deref(), endpoints.as_deref())?,
        Commands::List => cmd_list()?,
        Commands::Show { name } => cmd_show(&name)?,
        Commands::Remove { name } => cmd_remove(&name)?,
        Commands::Status => cmd_status()?,
        Commands::Logs { follow } => cmd_logs(follow)?,
        Commands::Cert { action } => match action {
            CertAction::Export { output } => cmd_cert_export(output)?,
        },
        Commands::Stop => cmd_stop()?,
        Commands::Templates => cmd_templates()?,
        Commands::Onboard => cmd_onboard().await?,
        Commands::Run { tool, args } => cmd_run(&tool, &args).await?,
    }

    Ok(())
}

/// Initialize the data directory, config file, and CA certificate
fn cmd_init() -> Result<()> {
    let config = AppConfig::default();
    let data_dir = config.data_dir();

    init_data_dir(&data_dir)?;

    // Generate CA certificate
    let _cert_manager = cert::CertManager::new(&data_dir)
        .with_context(|| "Failed to initialize CA certificate")?;

    // Save default config if it doesn't exist
    let config_path = AppConfig::config_path();
    if !config_path.exists() {
        config.save()?;
        println!("Created config file: {}", config_path.display());
    }

    println!("Initialized FakeKey at {}", data_dir.display());
    println!("\nDirectory structure:");
    println!("  {}/", data_dir.display());
    println!("  ├── config.json (real keys encrypted)");
    println!("  ├── certs/");
    println!("  │   ├── ca/");
    println!("  │   │   ├── cert.pem");
    println!("  │   │   └── key.pem (used for key encryption)");
    println!("  │   ├── cache/");
    println!("  │   └── ca.crt");
    println!("  ├── logs/");
    println!("  └── pid");
    println!("\nReal API keys are automatically encrypted using the CA private key.");
    println!("\nNext steps:");
    println!("  1. Add an API key:  fakekey add --name my-openai-key --key \"sk-...\" --template openai");
    println!("  2. Start the proxy: fakekey start");
    println!("  3. Trust the CA:    fakekey cert export");

    Ok(())
}

/// Start the proxy server
async fn cmd_start(port: u16, daemon_mode: bool) -> Result<()> {
    let mut config = AppConfig::load()?;
    config.proxy.port = port;
    let data_dir = config.data_dir();
    let pid_file = data_dir.join("pid");

    if daemon_mode && !daemon::is_daemon_mode() {
        daemon::daemonize(&pid_file)?;
    }

    let data_dir = config.data_dir();
    if !data_dir.exists() {
        anyhow::bail!(
            "FakeKey not initialized. Run `fakekey init` first."
        );
    }

    // Setup file logging
    let file_appender = tracing_appender::rolling::never(
        data_dir.join("logs"),
        "proxy.log",
    );
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    
    tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| {
                    tracing_subscriber::EnvFilter::new(&config.proxy.log_level)
                }),
        )
        .init();

    // Initialize audit logger first
    let audit_logger = match audit::AuditLogger::new(&data_dir) {
        Ok(logger) => {
            let logger = Arc::new(logger);
            let _ = logger.log(
                audit::AuditEventType::ProxyStart,
                format!("Proxy started on port {}", port),
                true,
            );
            Some(logger)
        }
        Err(e) => {
            println!("Warning: Failed to initialize audit logger: {}", e);
            None
        }
    };

    let cert_manager = Arc::new(
        cert::CertManager::new_with_logger(&data_dir, audit_logger.clone())
            .with_context(|| "Failed to load certificates")?,
    );

    let key_map = config.build_key_map();
    if key_map.is_empty() {
        println!("Warning: No API keys configured. Add keys with `fakekey add`.");
    } else {
        println!("Loaded {} API key mapping(s)", key_map.len());
    }

    // Write PID file
    let pid_file = data_dir.join("pid");
    std::fs::write(&pid_file, std::process::id().to_string())?;

    let state = Arc::new(proxy::ProxyState {
        key_map,
        cert_manager,
        allowed_hosts: config.proxy.allowed_hosts.clone(),
        audit_logger,
        config: Arc::new(config.clone()),
    });

    let addr: SocketAddr = format!("127.0.0.1:{}", port).parse()?;
    println!("Starting proxy on {}", addr);
    println!("Set your proxy to: http://127.0.0.1:{}", port);

    proxy::start_proxy(addr, state).await?;

    // Clean up PID file on exit
    let _ = std::fs::remove_file(&pid_file);

    Ok(())
}

/// Add a new API key
fn cmd_add(name: &str, key: &str, template: Option<&str>, header: Option<&str>, endpoints: Option<&str>) -> Result<()> {
    let mut config = AppConfig::load()?;

    // Check if name already exists
    if config.find_by_name(name).is_some() {
        anyhow::bail!(
            "Key name '{}' already exists. Remove it first with `fakekey remove --name {}`",
            name,
            name
        );
    }

    let existing_fake_keys: Vec<_> = config.api_keys.iter().map(|k| k.fake_key.as_str()).collect();
    let fake_key = generate_unique_fake_key(key, &existing_fake_keys);

    // Determine header name
    let header_name = if let Some(tpl) = template {
        if let Some(template_obj) = templates::get_template(tpl) {
            println!("Using template: {}", template_obj.description);
            template_obj.header_name.to_string()
        } else {
            anyhow::bail!("Template '{}' not found. Run `fakekey templates` to see available templates.", tpl);
        }
    } else if let Some(h) = header {
        h.to_string()
    } else {
        "Authorization".to_string()
    };

    // Determine endpoints
    let endpoints_list = if let Some(eps) = endpoints {
        // Parse comma-separated endpoints
        eps.split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    } else if let Some(tpl) = template {
        // Use template default endpoints
        if let Some(template_obj) = templates::get_template(tpl) {
            template_obj.default_endpoints.iter().map(|s| s.to_string()).collect()
        } else {
            vec![] // No template, empty endpoints
        }
    } else {
        vec![] // No template and no custom endpoints
    };

    let key_config = ApiKeyConfig {
        name: name.to_string(),
        real_key: key.to_string(),
        fake_key: fake_key.clone(),
        header_name: header_name.clone(),
        scan_locations: vec![ScanLocation::Header(header_name)],
        endpoints: endpoints_list,
        created_at: chrono::Utc::now(),
    };

    config.api_keys.push(key_config);
    config.save()?;

    // Log audit event
    if let Ok(data_dir) = std::env::var("FAKEKEY_DATA_DIR") {
        let data_dir_path = std::path::PathBuf::from(data_dir);
        if let Ok(logger) = audit::AuditLogger::new(&data_dir_path) {
            let _ = logger.log(
                audit::AuditEventType::KeyAdd,
                format!("Added key: {}", name),
                true,
            );
        }
    }

    println!("Added API key: {}", name);
    println!("Fake key: {}", fake_key);
    println!("\nUse this fake key in your applications instead of the real key.");
    println!("\nReal key is automatically encrypted using the CA private key.");

    Ok(())
}

/// List all configured API keys
fn cmd_list() -> Result<()> {
    let config = AppConfig::load()?;

    if config.api_keys.is_empty() {
        println!("No API keys configured.");
        println!("Add one with: fakekey add --name <name> --key <key> --template <template>");
        return Ok(());
    }

    println!("{:<20} {:<40} {:<20}", "NAME", "FAKE KEY", "HEADER");
    println!("{}", "-".repeat(80));

    for key in &config.api_keys {
        println!(
            "{:<20} {:<40} {:<20}",
            key.name, key.fake_key, key.header_name
        );
    }

    Ok(())
}

/// Show details for a specific key
fn cmd_show(name: &str) -> Result<()> {
    let config = AppConfig::load()?;

    match config.find_by_name(name) {
        Some(key) => {
            println!("Name:       {}", key.name);
            println!("Fake Key:   {}", key.fake_key);
            println!("Real Key:   {}", key_handler::mask_key(&key.real_key));
            println!("Header:     {}", key.header_name);
            println!("Created:    {}", key.created_at);
            println!("Scan locations:");
            for loc in &key.scan_locations {
                match loc {
                    ScanLocation::Header(name) => println!("  - Header: {}", name),
                    ScanLocation::UrlParam(name) => println!("  - URL Param: {}", name),
                    ScanLocation::JsonBody(path) => println!("  - JSON Body: {}", path),
                }
            }
        }
        None => {
            println!("Key '{}' not found.", name);
        }
    }

    Ok(())
}

/// Remove an API key configuration
fn cmd_remove(name: &str) -> Result<()> {
    let mut config = AppConfig::load()?;

    if config.remove_by_name(name) {
        config.save()?;
        println!("Removed API key: {}", name);
    } else {
        println!("Key '{}' not found.", name);
    }

    Ok(())
}

/// Check proxy status
fn cmd_status() -> Result<()> {
    let config = AppConfig::load()?;
    let data_dir = config.data_dir();
    let pid_file = data_dir.join("pid");

    if pid_file.exists() {
        let pid_str = std::fs::read_to_string(&pid_file)?;
        let pid: u32 = pid_str.trim().parse().unwrap_or_default();

        // Check if process is running
        if is_process_running(pid) {
            println!("Proxy status: RUNNING (PID: {})", pid);
            println!("Listen port:  {}", config.proxy.port);
            println!("API keys:     {} configured", config.api_keys.len());
            return Ok(());
        }
    }

    println!("Proxy status: STOPPED");
    println!("Start with:   fakekey start");

    Ok(())
}

/// View logs
fn cmd_logs(follow: bool) -> Result<()> {
    // TODO: Implement structured log reading from ~/.fakekey/logs/
    // For now, we output a hint about using tracing env vars
    let config = AppConfig::load()?;
    let log_dir = config.data_dir().join("logs");

    println!("Log directory: {}", log_dir.display());
    if follow {
        println!("Follow mode is not yet implemented.");
        println!("Hint: Set RUST_LOG=debug when running `fakekey start` for verbose output.");
    } else {
        let proxy_log = log_dir.join("proxy.log");
        if proxy_log.exists() {
            let content = std::fs::read_to_string(&proxy_log)?;
            print!("{}", content);
        } else {
            println!("No log files found. Start the proxy first.");
        }
    }

    Ok(())
}

/// Export CA certificate
fn cmd_cert_export(output: Option<String>) -> Result<()> {
    let config = AppConfig::load()?;
    let data_dir = config.data_dir();
    let ca_cert_path = data_dir.join("certs").join("ca.crt");

    if !ca_cert_path.exists() {
        anyhow::bail!("CA certificate not found. Run `fakekey init` first.");
    }

    let cert_pem = std::fs::read_to_string(&ca_cert_path)?;

    if let Some(output_path) = output {
        let output_path = config::expand_tilde(&output_path);
        std::fs::write(&output_path, &cert_pem)?;
        println!("CA certificate exported to: {}", output_path.display());
    } else {
        println!("CA certificate path: {}", ca_cert_path.display());
        println!();
        println!("To trust on macOS:");
        println!(
            "  sudo security add-trusted-cert -d -r trustRoot -k /Library/Keychains/System.keychain {}",
            ca_cert_path.display()
        );
        println!();
        println!("To trust on Linux:");
        println!(
            "  sudo cp {} /usr/local/share/ca-certificates/fakekey.crt && sudo update-ca-certificates",
            ca_cert_path.display()
        );
    }

    Ok(())
}

/// Stop the proxy server
fn cmd_stop() -> Result<()> {
    let config = AppConfig::load()?;
    let data_dir = config.data_dir();
    let pid_file = data_dir.join("pid");

    if !pid_file.exists() {
        println!("Proxy is not running (no PID file found).");
        return Ok(());
    }

    let pid_str = std::fs::read_to_string(&pid_file)?;
    let pid: u32 = pid_str.trim().parse().unwrap_or_default();

    if pid == 0 {
        std::fs::remove_file(&pid_file)?;
        println!("Removed stale PID file.");
        return Ok(());
    }

    // Send SIGTERM on Unix
    #[cfg(unix)]
    {
        use std::process::Command;
        let status = Command::new("kill").arg(pid.to_string()).status();
        match status {
            Ok(s) if s.success() => {
                println!("Sent stop signal to proxy (PID: {})", pid);
                std::fs::remove_file(&pid_file)?;
            }
            _ => {
                println!("Failed to stop proxy (PID: {}). It may have already exited.", pid);
                std::fs::remove_file(&pid_file)?;
            }
        }
    }

    #[cfg(not(unix))]
    {
        // TODO: Implement process termination on Windows
        println!("Stop is not implemented on this platform. Kill PID {} manually.", pid);
        std::fs::remove_file(&pid_file)?;
    }

    Ok(())
}

/// Check if a process is running by PID
fn is_process_running(pid: u32) -> bool {
    #[cfg(unix)]
    {
        use std::process::Command;
        Command::new("kill")
            .args(["-0", &pid.to_string()])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    #[cfg(not(unix))]
    {
        // TODO: Implement process check on Windows
        false
    }
}

/// List available service templates
fn cmd_templates() -> Result<()> {
    println!("{:<15} {:<50}", "SERVICE", "DESCRIPTION");
    println!("{}", "-".repeat(65));

    for template in templates::list_templates() {
        println!(
            "{:<15} {:<50}",
            template.name, template.description
        );
    }

    println!("\nUsage:");
    println!("  fakekey add --name my-openai-key --key \"sk-...\" --template openai");

    Ok(())
}

/// Detect the user's shell and return (shell_name, rc_file_path)
fn detect_shell_and_rc() -> (String, std::path::PathBuf) {
    let home = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    let shell = std::env::var("SHELL").unwrap_or_default();

    if shell.contains("zsh") {
        ("zsh".to_string(), home.join(".zshrc"))
    } else if shell.contains("fish") {
        ("fish".to_string(), home.join(".config/fish/config.fish"))
    } else if shell.contains("bash") {
        // On macOS, prefer .bash_profile; on Linux prefer .bashrc
        let rc = if cfg!(target_os = "macos") {
            if home.join(".bash_profile").exists() {
                home.join(".bash_profile")
            } else {
                home.join(".bashrc")
            }
        } else {
            home.join(".bashrc")
        };
        ("bash".to_string(), rc)
    } else {
        // Fallback: use .profile
        ("sh".to_string(), home.join(".profile"))
    }
}

/// Setup shell environment variables for CA certificate trust.
/// Adds NODE_EXTRA_CA_CERTS, SSL_CERT_FILE, REQUESTS_CA_BUNDLE to the shell RC file.
/// Returns Ok(true) if changes were made, Ok(false) if already configured.
fn setup_shell_env_vars(ca_cert_path: &std::path::Path) -> Result<bool> {
    use std::io::Write;

    let (shell_name, rc_path) = detect_shell_and_rc();
    let ca_path_str = ca_cert_path.to_string_lossy();

    // Marker to identify our block
    let marker = "# >>> fakekey CA certificate environment variables >>>";
    let marker_end = "# <<< fakekey CA certificate environment variables <<<";

    // Check if already configured
    if rc_path.exists() {
        let content = std::fs::read_to_string(&rc_path)
            .with_context(|| format!("Failed to read {}", rc_path.display()))?;
        if content.contains(marker) {
            println!("   ✅ Environment variables already configured in {}", rc_path.display());
            return Ok(false);
        }
    }

    // Generate the env block based on shell type
    let env_block = if shell_name == "fish" {
        format!(
            r#"
{marker}
set -gx NODE_EXTRA_CA_CERTS "{ca_path}"
set -gx SSL_CERT_FILE "{ca_path}"
set -gx REQUESTS_CA_BUNDLE "{ca_path}"
{marker_end}
"#,
            marker = marker,
            marker_end = marker_end,
            ca_path = ca_path_str,
        )
    } else {
        format!(
            r#"
{marker}
export NODE_EXTRA_CA_CERTS="{ca_path}"
export SSL_CERT_FILE="{ca_path}"
export REQUESTS_CA_BUNDLE="{ca_path}"
{marker_end}
"#,
            marker = marker,
            marker_end = marker_end,
            ca_path = ca_path_str,
        )
    };

    // Ensure parent directory exists (for fish)
    if let Some(parent) = rc_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Append to RC file
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&rc_path)
        .with_context(|| format!("Failed to open {} for writing", rc_path.display()))?;

    file.write_all(env_block.as_bytes())
        .with_context(|| format!("Failed to write to {}", rc_path.display()))?;

    println!("   ✅ Added environment variables to {}", rc_path.display());
    println!();
    println!("   The following variables were added:");
    println!("   • NODE_EXTRA_CA_CERTS  — for Node.js (Claude Code, VS Code, etc.)");
    println!("   • SSL_CERT_FILE        — for Go, Ruby, and other tools");
    println!("   • REQUESTS_CA_BUNDLE   — for Python requests library");
    println!();
    println!("   🔄 To apply now, run:");
    if shell_name == "fish" {
        println!("      source {}", rc_path.display());
    } else {
        println!("      source {}", rc_path.display());
    }

    Ok(true)
}

/// Interactive setup wizard
async fn cmd_onboard() -> Result<()> {
    println!("🚀 Welcome to FakeKey Interactive Setup!");
    println!("This wizard will help you set up everything in one go.");
    println!();

    // Step 1: Check if already initialized
    let config = AppConfig::load()?;
    let data_dir = config.data_dir();
    let is_initialized = data_dir.exists() && data_dir.join("certs/ca/cert.pem").exists();
    
    if is_initialized {
        println!("✅ FakeKey is already initialized at {}", data_dir.display());
        println!("   You can continue to add keys or start the proxy.");
        println!();
    } else {
        println!("📁 Step 1: Initializing FakeKey...");
        cmd_init()?;
        println!("✅ Initialization complete!");
        println!();
    }

    // Step 2: Trust CA certificate
    println!("🔐 Step 2: Certificate Setup");
    println!("   FakeKey needs to generate a CA certificate to intercept HTTPS traffic.");
    println!("   You need to trust this certificate on your system.");
    println!();
    
    println!("📍 CA certificate location: {}/certs/ca.crt", data_dir.display());
    println!();
    
    println!("🍎 macOS (run this in a separate terminal):");
    println!("   sudo security add-trusted-cert -d -r trustRoot \\");
    println!("     -k /Library/Keychains/System.keychain {}/certs/ca.crt", data_dir.display());
    println!();
    
    println!("🐧 Linux (run this in a separate terminal):");
    println!("   sudo cp {}/certs/ca.crt /usr/local/share/ca-certificates/fakekey.crt", data_dir.display());
    println!("   sudo update-ca-certificates");
    println!();
    
    println!("🪟 Windows:");
    println!("   1. Run: certmgr.msc");
    println!("   2. Go to Trusted Root Certification Authorities → Certificates");
    println!("   3. Right-click → All Tasks → Import");
    println!("   4. Select: {}/certs/ca.crt", data_dir.display());
    println!();
    
    print!("Have you trusted the CA certificate? (y/N): ");
    use std::io;
    use std::io::Write;
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    if !input.trim().to_lowercase().starts_with('y') {
        println!("⚠️  Please trust the CA certificate before continuing.");
        println!("   HTTPS requests will fail without a trusted certificate.");
        println!();
    } else {
        println!("✅ Certificate trusted!");
        println!();
    }

    // Step 2b: Setup shell environment variables for CA trust
    println!("🌐 Configuring shell environment for CA certificate trust...");
    let (shell_name, rc_path) = detect_shell_and_rc();
    println!("   Detected shell: {} ({})", shell_name, rc_path.display());
    let ca_cert_path = data_dir.join("certs").join("ca.crt");
    match setup_shell_env_vars(&ca_cert_path) {
        Ok(true) => {
            println!();
        }
        Ok(false) => {
            // Already configured, message printed inside the function
            println!();
        }
        Err(e) => {
            println!("   ⚠️  Failed to auto-configure shell environment: {}", e);
            println!("   You can manually add these to your {}:", rc_path.display());
            println!("      export NODE_EXTRA_CA_CERTS=\"{}\"", ca_cert_path.display());
            println!("      export SSL_CERT_FILE=\"{}\"", ca_cert_path.display());
            println!("      export REQUESTS_CA_BUNDLE=\"{}\"", ca_cert_path.display());
            println!();
        }
    }

    // Step 3: Add API keys
    println!("🔑 Step 3: Add API Keys");
    println!("   Let's add your first API key.");
    println!();
    
    loop {
        println!("Available templates:");
        for template in templates::list_templates() {
            println!("  - {}: {}", template.name, template.description);
        }
        println!();
        
        print!("Enter template name (or 'custom' for custom header, or 'done' to finish): ");
        io::stdout().flush()?;
        let mut template_input = String::new();
        io::stdin().read_line(&mut template_input)?;
        let template_input = template_input.trim();
        
        if template_input.to_lowercase() == "done" {
            break;
        }
        
        if template_input.to_lowercase() == "custom" {
            print!("Enter key name: ");
            io::stdout().flush()?;
            let mut name = String::new();
            io::stdin().read_line(&mut name)?;
            let name = name.trim();
            
            if name.is_empty() {
                println!("❌ Key name cannot be empty!");
                continue;
            }
            
            print!("Enter real API key: ");
            io::stdout().flush()?;
            let mut key = String::new();
            io::stdin().read_line(&mut key)?;
            let key = key.trim();
            
            if key.is_empty() {
                println!("❌ API key cannot be empty!");
                continue;
            }
            
            print!("Enter header name (default: Authorization): ");
            io::stdout().flush()?;
            let mut header = String::new();
            io::stdin().read_line(&mut header)?;
            let header = header.trim();
            let header = if header.is_empty() { "Authorization" } else { header };
            
            // Add custom key
            let mut config = AppConfig::load()?;
            if config.find_by_name(name).is_some() {
                println!("❌ Key name '{}' already exists!", name);
                continue;
            }
            
            let existing_fake_keys: Vec<_> = config.api_keys.iter().map(|k| k.fake_key.as_str()).collect();
            let fake_key = config::generate_unique_fake_key(key, &existing_fake_keys);
            
            let key_config = config::ApiKeyConfig {
                name: name.to_string(),
                real_key: key.to_string(),
                fake_key: fake_key.clone(),
                header_name: header.to_string(),
                scan_locations: vec![config::ScanLocation::Header(header.to_string())],
                endpoints: vec![], // Empty endpoints for custom keys
                created_at: chrono::Utc::now(),
            };
            
            config.api_keys.push(key_config);
            config.save()?;
            
            println!("✅ Added key: {}", name);
            println!("   Fake key: {}", fake_key);
            println!();
        } else {
            let template = match templates::get_template(template_input) {
                Some(t) => t,
                None => {
                    println!("❌ Unknown template: {}", template_input);
                    continue;
                }
            };
            
            print!("Enter key name: ");
            io::stdout().flush()?;
            let mut name = String::new();
            io::stdin().read_line(&mut name)?;
            let name = name.trim();
            
            if name.is_empty() {
                println!("❌ Key name cannot be empty!");
                continue;
            }
            
            print!("Enter real API key: ");
            io::stdout().flush()?;
            let mut key = String::new();
            io::stdin().read_line(&mut key)?;
            let key = key.trim();
            
            if key.is_empty() {
                println!("❌ API key cannot be empty!");
                continue;
            }
            
            // Add templated key
            let mut config = AppConfig::load()?;
            if config.find_by_name(name).is_some() {
                println!("❌ Key name '{}' already exists!", name);
                continue;
            }
            
            let existing_fake_keys: Vec<_> = config.api_keys.iter().map(|k| k.fake_key.as_str()).collect();
            let fake_key = config::generate_unique_fake_key(key, &existing_fake_keys);
            
            let key_config = config::ApiKeyConfig {
                name: name.to_string(),
                real_key: key.to_string(),
                fake_key: fake_key.clone(),
                header_name: template.header_name.to_string(),
                scan_locations: vec![config::ScanLocation::Header(template.header_name.to_string())],
                endpoints: template.default_endpoints.iter().map(|s| s.to_string()).collect(),
                created_at: chrono::Utc::now(),
            };
            
            config.api_keys.push(key_config);
            config.save()?;
            
            println!("✅ Added key: {} (using {} template)", name, template.name);
            println!("   Fake key: {}", fake_key);
            println!();
        }
    }

    // Step 4: Show configuration summary
    let config = AppConfig::load()?;
    if !config.api_keys.is_empty() {
        println!("📋 Configuration Summary:");
        println!("   Data directory: {}", data_dir.display());
        println!("   Config file: {}/config.json", data_dir.display());
        println!("   CA certificate: {}/certs/ca.crt", data_dir.display());
        println!();
        println!("   Configured keys:");
        for key in &config.api_keys {
            println!("   - {} (fake: {})", key.name, key.fake_key);
        }
        println!();
    }

    // Step 4: Start proxy
    println!("🚀 Step 4: Start Proxy");
    println!("   Ready to start the proxy server!");
    println!();
    
    // Check if proxy is already running
    let data_dir = config.data_dir();
    let pid_file = data_dir.join("pid");
    let mut is_running = false;
    
    if pid_file.exists() {
        if let Ok(pid_str) = std::fs::read_to_string(&pid_file) {
            if let Ok(pid) = pid_str.trim().parse::<u32>() {
                if is_process_running(pid) {
                    is_running = true;
                    println!("⚠️  Proxy is already running (PID: {})", pid);
                    println!("   Listen port:  {}", config.proxy.port);
                }
            }
        }
    }
    
    if is_running {
        print!("Restart proxy? (Y/n): ");
        io::stdout().flush()?;
        let mut restart_input = String::new();
        io::stdin().read_line(&mut restart_input)?;
        
        if restart_input.trim().to_lowercase() != "n" {
            println!("🔄 Restarting proxy...");
            cmd_stop()?;
            // Give it a moment to stop
            std::thread::sleep(std::time::Duration::from_millis(500));
            println!("🎉 Starting proxy in background...");
            println!("   Proxy will run on port {}", config.proxy.port);
            println!("   Use 'fakekey stop' to stop the proxy.");
            println!();
            
            // Start proxy in background (daemon mode)
            cmd_start(config.proxy.port, true).await?;
        } else {
            println!("💡 Proxy continues running. You can restart it later with:");
            println!("   fakekey stop && fakekey start");
            println!("   fakekey restart  # (if implemented)");
        }
    } else {
        print!("Start proxy now? (Y/n): ");
        io::stdout().flush()?;
        let mut start_input = String::new();
        io::stdin().read_line(&mut start_input)?;
        
        if start_input.trim().to_lowercase() != "n" {
            println!("🎉 Starting proxy in background...");
            println!("   Proxy will run on port {}", config.proxy.port);
            println!("   Use 'fakekey stop' to stop the proxy.");
            println!();
            
            // Start proxy in background (daemon mode)
            cmd_start(config.proxy.port, true).await?;
        } else {
            println!("💡 You can start the proxy later with:");
            println!("   fakekey start --daemon");
            println!("   fakekey start  # foreground mode");
        }
    }

    println!();
    println!("🎊 Setup complete! Your API keys are now protected with FakeKey.");
    println!("   Use fake keys in your applications instead of real keys.");
    println!();
    println!("📚 Need help? Check the documentation or run: fakekey --help");

    Ok(())
}

/// Run a CLI tool with proxy automatically configured
async fn cmd_run(tool_name: &str, args: &[String]) -> Result<()> {
    let tool = tool_launcher::get_tool(tool_name)
        .ok_or_else(|| anyhow::anyhow!("Unknown tool: {}", tool_name))?;
    
    let config = AppConfig::load()?;
    let data_dir = config.data_dir();
    
    if !data_dir.exists() {
        anyhow::bail!(
            "FakeKey not initialized. Run `fakekey init` first."
        );
    }
    
    let ca_cert_path = data_dir.join("certs").join("ca.crt");
    if !ca_cert_path.exists() {
        anyhow::bail!(
            "CA certificate not found. Run `fakekey init` first."
        );
    }
    
    let pid_file = data_dir.join("pid");
    let mut proxy_running = false;
    
    if pid_file.exists() {
        if let Ok(pid_str) = std::fs::read_to_string(&pid_file) {
            if let Ok(pid) = pid_str.trim().parse::<u32>() {
                if is_process_running(pid) {
                    proxy_running = true;
                    println!("✅ Proxy is running (PID: {})", pid);
                }
            }
        }
    }
    
    if !proxy_running {
        println!("🔄 Proxy is not running. Starting proxy in background...");
        
        use std::process::Command;
        let current_exe = std::env::current_exe()
            .with_context(|| "Failed to get current executable path")?;
        
        let mut child = Command::new(current_exe)
            .arg("start")
            .arg("--daemon")
            .arg("--port")
            .arg(config.proxy.port.to_string())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .with_context(|| "Failed to start proxy")?;
        
        child.wait()
            .with_context(|| "Failed to wait for proxy startup")?;
        
        std::thread::sleep(std::time::Duration::from_secs(1));
        
        if pid_file.exists() {
            if let Ok(pid_str) = std::fs::read_to_string(&pid_file) {
                if let Ok(pid) = pid_str.trim().parse::<u32>() {
                    if is_process_running(pid) {
                        println!("✅ Proxy started successfully (PID: {})", pid);
                        proxy_running = true;
                    }
                }
            }
        }
        
        if !proxy_running {
            anyhow::bail!("Failed to start proxy. Try running `fakekey start --daemon` manually.");
        }
    }
    
    tool_launcher::launch_tool(tool, args, config.proxy.port, &ca_cert_path)?;
    
    Ok(())
}

