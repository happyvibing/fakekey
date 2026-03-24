mod audit;
mod cert;
mod cli;
mod config;
mod daemon;
mod key_handler;
mod proxy;
mod security;
mod templates;

use anyhow::{Context, Result};
use clap::Parser;
use cli::{CertAction, Cli, Commands};
use config::{generate_unique_fake_key, init_data_dir, AppConfig, ApiKeyConfig, ScanLocation};
use std::net::SocketAddr;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize tracing for non-start commands
    if !matches!(cli.command, Commands::Start { .. }) {
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
            service,
            key,
            header,
            template,
        } => cmd_add(&service, &key, &header, template)?,
        Commands::List => cmd_list()?,
        Commands::Show { service } => cmd_show(&service)?,
        Commands::Remove { service } => cmd_remove(&service)?,
        Commands::Status => cmd_status()?,
        Commands::Logs { follow } => cmd_logs(follow)?,
        Commands::Cert { action } => match action {
            CertAction::Export { output } => cmd_cert_export(output)?,
        },
        Commands::Stop => cmd_stop()?,
        Commands::Templates => cmd_templates()?,
        Commands::Encrypt { enable } => cmd_encrypt(enable)?,
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
    println!("  ├── config.yaml");
    println!("  ├── certs/");
    println!("  │   ├── ca/");
    println!("  │   │   ├── cert.pem");
    println!("  │   │   └── key.pem");
    println!("  │   ├── cache/");
    println!("  │   └── ca.crt");
    println!("  ├── logs/");
    println!("  └── pid");
    println!("\nNext steps:");
    println!("  1. Add an API key:  fakekey add --service openai --key \"sk-...\"");
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
fn cmd_add(service: &str, key: &str, header: &str, use_template: bool) -> Result<()> {
    let mut config = AppConfig::load()?;

    // Check if service already exists
    if config.find_by_service(service).is_some() {
        anyhow::bail!(
            "Service '{}' already exists. Remove it first with `fakekey remove --service {}`",
            service,
            service
        );
    }

    let existing_fake_keys: Vec<_> = config.api_keys.iter().map(|k| k.fake_key.as_str()).collect();
    let fake_key = generate_unique_fake_key(key, &existing_fake_keys);

    let key_config = if use_template {
        if let Some(template) = templates::get_template(service) {
            println!("Using template: {}", template.description);
            let mut config = template.to_api_key_config(key.to_string(), fake_key.clone());
            config.scan_locations = vec![ScanLocation::Header(template.header_name.to_string())];
            config
        } else {
            println!("No template found for '{}', using default configuration", service);
            ApiKeyConfig {
                service: service.to_string(),
                real_key: key.to_string(),
                fake_key: fake_key.clone(),
                header_name: header.to_string(),
                scan_locations: vec![ScanLocation::Header(header.to_string())],
                created_at: chrono::Utc::now(),
            }
        }
    } else {
        ApiKeyConfig {
            service: service.to_string(),
            real_key: key.to_string(),
            fake_key: fake_key.clone(),
            header_name: header.to_string(),
            scan_locations: vec![ScanLocation::Header(header.to_string())],
            created_at: chrono::Utc::now(),
        }
    };

    config.api_keys.push(key_config);
    config.save()?;

    // Log audit event
    if let Ok(data_dir) = std::env::var("FAKEKEY_DATA_DIR") {
        let data_dir_path = std::path::PathBuf::from(data_dir);
        if let Ok(logger) = audit::AuditLogger::new(&data_dir_path) {
            let _ = logger.log(
                audit::AuditEventType::KeyAdd,
                format!("Added key for service: {}", service),
                true,
            );
        }
    }

    println!("Added API key for service: {}", service);
    println!("Fake key: {}", fake_key);
    println!("\nUse this fake key in your applications instead of the real key.");

    Ok(())
}

/// List all configured API keys
fn cmd_list() -> Result<()> {
    let config = AppConfig::load()?;

    if config.api_keys.is_empty() {
        println!("No API keys configured.");
        println!("Add one with: fakekey add --service <name> --key <key>");
        return Ok(());
    }

    println!("{:<15} {:<40} {:<20}", "SERVICE", "FAKE KEY", "HEADER");
    println!("{}", "-".repeat(75));

    for key in &config.api_keys {
        println!(
            "{:<15} {:<40} {:<20}",
            key.service, key.fake_key, key.header_name
        );
    }

    Ok(())
}

/// Show details for a specific service
fn cmd_show(service: &str) -> Result<()> {
    let config = AppConfig::load()?;

    match config.find_by_service(service) {
        Some(key) => {
            println!("Service:    {}", key.service);
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
            println!("Service '{}' not found.", service);
        }
    }

    Ok(())
}

/// Remove an API key configuration
fn cmd_remove(service: &str) -> Result<()> {
    let mut config = AppConfig::load()?;

    if config.remove_by_service(service) {
        config.save()?;
        println!("Removed API key for service: {}", service);
    } else {
        println!("Service '{}' not found.", service);
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
    println!("{:<15} {:<20} {:<50}", "SERVICE", "KEY PATTERN", "DESCRIPTION");
    println!("{}", "-".repeat(85));

    for template in templates::list_templates() {
        println!(
            "{:<15} {:<20} {:<50}",
            template.name, template.key_pattern, template.description
        );
    }

    println!("\nUse --template flag when adding a key:");
    println!("  fakekey add --service openai --key \"sk-...\" --template");

    Ok(())
}

/// Enable or disable config encryption
fn cmd_encrypt(enable: bool) -> Result<()> {
    let mut config = AppConfig::load()?;

    if enable {
        if config.security.encrypt_config {
            println!("Config encryption is already enabled.");
            return Ok(());
        }

        println!("Enabling config encryption...");
        println!("Set FAKEKEY_PASSWORD environment variable for encryption/decryption.");

        config.security.encrypt_config = true;
        config.save()?;

        println!("Config encryption enabled.");
        println!("The config file will be encrypted on the next save.");
    } else {
        if !config.security.encrypt_config {
            println!("Config encryption is already disabled.");
            return Ok(());
        }

        println!("Disabling config encryption...");

        config.security.encrypt_config = false;
        config.save()?;

        println!("Config encryption disabled.");
        println!("The config file is now saved in plain text.");
    }

    Ok(())
}
