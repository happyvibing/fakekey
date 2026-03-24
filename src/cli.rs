use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "fakekey", about = "API Key Proxy Agent", version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize configuration and generate CA certificate
    Init,

    /// Start the proxy server
    Start {
        /// Port to listen on
        #[arg(short, long, default_value_t = 1157)]
        port: u16,

        /// Run as daemon in background
        #[arg(short, long)]
        daemon: bool,
    },

    /// Add an API key
    Add {
        /// Service name (e.g., openai, github)
        #[arg(short, long)]
        service: String,

        /// Real API key
        #[arg(short, long)]
        key: String,

        /// Header name for the key
        #[arg(long, default_value = "Authorization")]
        header: String,
    },

    /// List all configured API keys
    List,

    /// Show details for a specific service
    Show {
        /// Service name
        #[arg(short, long)]
        service: String,
    },

    /// Remove an API key configuration
    Remove {
        /// Service name
        #[arg(short, long)]
        service: String,
    },

    /// Check proxy status
    Status,

    /// View logs
    Logs {
        /// Follow log output
        #[arg(short, long)]
        follow: bool,
    },

    /// Certificate management
    Cert {
        #[command(subcommand)]
        action: CertAction,
    },

    /// Stop the proxy server
    Stop,
}

#[derive(Subcommand)]
pub enum CertAction {
    /// Export CA certificate
    Export {
        /// Output path
        #[arg(short, long)]
        output: Option<String>,
    },
}
