<p align="center">
 English | <b><a href="README_CN.md">中文</a></b> 
</p>

# FakeKey - API Key Proxy Agent

In the era of AI Agents like Openclaw, ClaudeCode, etc., we have to expose various service API Tokens directly in environment variables. Your api_key will be inserted into context and known by model service providers, known by the lobsters you trust, perhaps captured and read by some skill, and even more likely to be directly learned by strangers asking your lobster. There are too many leak cases, I cannot trust to bind my credit card-linked api_key directly exposed to any Agent and local environment variables, so FakeKey was created, the safest measure is to never expose the real api_key.

FakeKey is a high-performance API key proxy program developed in Rust. Through intelligent proxy technology, it can automatically replace fake keys with real keys in HTTP headers and URLs without exposing real credentials, while maintaining complete HTTP API compatibility and performance.

## How It Works

```
┌─────────────────┐         ┌──────────────────────────┐         ┌─────────────────┐
│   Client Agent  │ HTTP/S  │       FakeKey Proxy      │ HTTP/S  │  External API   │
│                 │────────▶│  1. TLS Decryption       │────────▶│                 │
│  Uses fake key  │         │  2. Identify replace key │         │  Get real key   │
│  sk-xxx_fk      │         │  3. Forward request      │         │  sk-xxx         │
└─────────────────┘         └──────────────────────────┘         └─────────────────┘
```


## Quick Start

### Installation

#### Quick Install (macOS / Linux)

```bash
curl -fsSL https://raw.githubusercontent.com/happyvibing/fakekey/main/install.sh | bash
```

#### Homebrew (macOS / Linux)

```bash
brew install happyvibing/tap/fakekey
```

#### Cargo (from crates.io)

```bash
cargo install fakekey
```

#### Install from Source

```bash
git clone https://github.com/happyvibing/fakekey.git
cd fakekey
cargo build --release
cargo install --path .
```

#### Download Pre-built Binary

Pre-built binaries for all platforms are available on the [GitHub Releases](https://github.com/happyvibing/fakekey/releases) page:

| Platform | File |
|----------|------|
| macOS (Apple Silicon) | `fakekey-macos-arm64.tar.gz` |
| macOS (Intel) | `fakekey-macos-amd64.tar.gz` |
| Linux (x86_64) | `fakekey-linux-amd64.tar.gz` |
| Linux (ARM64) | `fakekey-linux-arm64.tar.gz` |
| Windows (x86_64) | `fakekey-windows-amd64.zip` |

### One-Click Initialization

```bash
fakekey onboard
```

During the process, you'll be prompted to trust the CA certificate. For first-time use, you need to add the CA certificate to the system trust list:

```bash
# macOS
sudo security add-trusted-cert -d -r trustRoot \
  -k /Library/Keychains/System.keychain ~/.fakekey/certs/ca.crt

# Linux
sudo cp ~/.fakekey/certs/ca.crt /usr/local/share/ca-certificates/fakekey.crt
sudo update-ca-certificates
```

### Basic Commands

```bash
# Generate OpenAI type fake KEY
fakekey add --name my-openai-key --key "sk-proj-xxxxx" --template openai

# Generate custom fake KEY
fakekey add --name my-custom --key "xxxxx"

# View available templates
fakekey templates

# List all configured keys
fakekey list

# View specific key configuration
fakekey show --name my-openai-key

# Delete key configuration
fakekey remove --name my-openai-key

# View status
fakekey status

# Run (default background)
fakekey start

# Stop
fakekey stop

# View logs
fakekey logs
```

### One-Click Tool Launch (Recommended)

FakeKey provides a convenient way to launch CLI tools with automatic proxy protection:

```bash
# Launch Claude Code with automatic proxy protection
fakekey run claude

# Launch OpenClaw with automatic proxy protection
fakekey run openclaw

# Pass additional arguments to the tool
fakekey run claude --help
```

This command automatically completes the following operations:
1. Check if the proxy is running, start it automatically if not
2. Set all necessary environment variables (HTTP_PROXY, HTTPS_PROXY, NODE_EXTRA_CA_CERTS, etc.)
3. Launch the tool with proxy protection enabled
4. All your API keys will be automatically protected!

### Manual Proxy Configuration

If you prefer manual configuration:

- Replace real API keys with generated fake keys in your Agent or application
- Set the network proxy to `http://127.0.0.1:1155` in your Agent or application

For example, first set the network proxy:
```bash
export http_proxy=http://127.0.0.1:1155
export https_proxy=http://127.0.0.1:1155
export NODE_EXTRA_CA_CERTS=~/.fakekey/certs/ca.crt
```
Then launch your Agent such as `claude`, `openclaw`, `pi`

## Security

1. **Key Protection** - Real API keys are encrypted with AES-256-GCM and stored locally in config files; the encryption key is securely stored in OS-level key storage (macOS Keychain / Linux Secret Service / Windows Credential Manager)
2. **Certificate Security** - Locally generated CA certificates with private key file permissions 0600, used for TLS MITM proxy
3. **Network Security** - Only listens on local 127.0.0.1, supports host whitelist
4. **Log Desensitization** - Automatically hides sensitive information
5. **Audit Trail** - All key operations are logged to audit logs


## License

Apache License 2.0

## Contributing

Issues and Pull Requests are welcome!
