use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "fakekey")]
#[command(about = "API Key Proxy Agent")]
#[command(version)]
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
        #[arg(short, long, default_value_t = 1155)]
        port: u16,

        /// Run in foreground (default is background)
        #[arg(short, long)]
        foreground: bool,
    },

    /// Add an API key
    Add {
        /// Unique name for this key (e.g., my-openai-key)
        #[arg(short, long)]
        name: String,

        /// Real API key
        #[arg(short, long)]
        key: String,

        /// Use template (e.g., openai, anthropic, github)
        #[arg(short, long)]
        template: Option<String>,

        /// Custom endpoints (comma-separated, e.g., api.openai.com,custom.example.com)
        #[arg(long)]
        endpoints: Option<String>,
    },

    /// List all configured API keys
    List,

    /// Show details for a specific key
    Show {
        /// Key name
        #[arg(short, long)]
        name: String,
    },

    /// Remove an API key configuration
    Remove {
        /// Key name
        #[arg(short, long)]
        name: String,
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

    /// List available service templates
    Templates,

    /// Interactive setup wizard
    Onboard,

    /// Run a CLI tool with proxy automatically configured
    Run {
        /// Tool name (e.g., claude, openclaw)
        tool: String,

        /// Additional arguments to pass to the tool
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
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
