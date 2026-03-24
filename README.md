# FakeKey - API Key Proxy Agent

FakeKey is a Rust-based CLI proxy application for managing and replacing API keys. By setting up a network proxy, applications can use fake keys, and FakeKey will automatically replace them with real keys in requests, thereby protecting sensitive credentials.

## Core Features

### ✅ Implemented Features

- **HTTP/HTTPS Proxy** - Supports MITM-style HTTPS traffic decryption
- **Key Management** - Add, list, view, delete API key configurations
- **Fake Key Generation** - Automatically generates fake keys of the same length as original keys (with `_fk` suffix)
- **Key Replacement** - Automatically replaces fake keys with real keys in headers, URL parameters, and request bodies
- **Certificate Management** - Automatically generates and manages CA certificates and server certificates
- **Configuration Encryption** - Uses AES-256-GCM encryption to protect configuration files
- **Log Desensitization** - Automatically hides sensitive information in logs
- **Audit Logging** - Records all critical operations to audit logs
- **Service Templates** - Pre-configured templates for common services like OpenAI, GitHub, Anthropic
- **Daemon Mode** - Supports background running
- **Complete Testing** - Includes unit tests and integration tests

## Quick Start

### Installation

```bash
# Build from source
cargo build --release

# Install to system
cargo install --path .
```

### Initialization

```bash
# Initialize configuration and CA certificates
fakekey init

# Output:
# Initialized FakeKey at ~/.fakekey
# Directory structure:
#   ~/.fakekey/
#   ├── config.yaml
#   ├── certs/
#   │   ├── ca/
#   │   │   ├── cert.pem
#   │   │   └── key.pem
#   │   ├── cache/
#   │   └── ca.crt
#   ├── logs/
#   └── pid
```

### Add API Keys

```bash
# Add OpenAI key using preset template
fakekey add --service openai --key "sk-proj-xxxxx" --template

# Output:
# Using template: OpenAI API (api.openai.com)
# Added API key for service: openai
# Fake key: sk-proj-xxxxx_fk
```

### View Available Templates

```bash
fakekey templates

# Output:
# SERVICE         KEY PATTERN          DESCRIPTION
# -------------------------------------------------------------------------------
# openai          sk-                  OpenAI API (api.openai.com)
# anthropic       sk-ant-              Anthropic Claude API (api.anthropic.com)
# github          ghp_                 GitHub Personal Access Token (api.github.com)
# google          AIza                 Google Cloud API (googleapis.com)
# huggingface     hf_                  Hugging Face API (huggingface.co)
# deepseek        sk-                  DeepSeek API (api.deepseek.com)
```

### Start Proxy

```bash
# Run in foreground
fakekey start

# Run in background (daemon mode)
fakekey start --daemon

# Specify port
fakekey start --port 8080
```

### Trust CA Certificate

On first use, you need to add the CA certificate to the system trust list:

```bash
# Export CA certificate
fakekey cert export

# macOS
sudo security add-trusted-cert -d -r trustRoot \
  -k /Library/Keychains/System.keychain ~/.fakekey/certs/ca.crt

# Linux
sudo cp ~/.fakekey/certs/ca.crt /usr/local/share/ca-certificates/fakekey.crt
sudo update-ca-certificates
```

### Configure Application

In your application, set:
- **API Key**: Use fake key `sk-proj-xxxxx_fk`
- **Proxy**: `http://127.0.0.1:1157`

## CLI Commands

### Basic Commands

```bash
# List all configured keys
fakekey list

# View specific service configuration
fakekey show --service openai

# Remove key configuration
fakekey remove --service openai

# View proxy status
fakekey status

# Stop proxy
fakekey stop

# View logs
fakekey logs
```

### Configuration Encryption

```bash
# Enable configuration encryption
export FAKEKEY_PASSWORD="your-secure-password"
fakekey encrypt --enable

# Disable configuration encryption
fakekey encrypt --disable
```

## Configuration File Example

`~/.fakekey/config.yaml`:

```yaml
proxy:
  port: 1157
  log_level: info
  data_dir: "~/.fakekey"
  allowed_hosts:
    - api.openai.com
    - api.anthropic.com

api_keys:
  - service: openai
    real_key: "sk-proj-real-key-here"
    fake_key: "sk-proj-real-key-h_fk"
    header_name: "Authorization"
    scan_locations:
      - type: header
        name: Authorization
    created_at: "2024-03-25T00:00:00Z"

security:
  encrypt_config: false
```

## Architecture Design

```
┌─────────────────┐         ┌──────────────────────────┐         ┌─────────────────┐
│   Client App    │ HTTPS   │   FakeKey Proxy          │ HTTPS   │  External API   │
│                 │────────▶│  1. TLS Decryption       │────────▶│                 │
│  Uses Fake Key  │         │  2. Identify & Replace   │         │  Receives Real  │
│  sk-xxx_fk      │         │  3. Forward Request      │         │  Key sk-xxx     │
└─────────────────┘         └──────────────────────────┘         └─────────────────┘
```

## Module Description

- **config** - Configuration management and fake key generation
- **proxy** - HTTP/HTTPS proxy server
- **cert** - CA certificate and server certificate management
- **key_handler** - Key identification and replacement logic
- **security** - Configuration encryption and data desensitization
- **audit** - Audit logging
- **templates** - Pre-configured service templates
- **daemon** - Background process management

## Testing

```bash
# Run all tests
cargo test

# Run specific tests
cargo test test_key_replacement

# View test coverage
cargo test --verbose
```

Tests include:
- 17 unit tests
- 6 integration tests
- Coverage of key replacement, configuration management, encryption, templates and other core functions

## Security Considerations

1. **Key Protection** - Real keys are stored locally only, with optional encrypted storage
2. **Certificate Security** - Locally generated CA certificates, private key files with 0600 permissions
3. **Network Security** - Only listens on local 127.0.0.1, supports host whitelist
4. **Log Desensitization** - Automatically hides sensitive information
5. **Audit Trail** - All critical operations recorded to audit logs

## Use Cases

### IDE Integration Development

```bash
# Configure FakeKey
fakekey add --service openai --key "sk-real-key" --template
fakekey start --daemon

# Configure in IDE
# API Key: sk-real-key_fk
# Proxy: http://127.0.0.1:1157
```

### CI/CD Environment

```bash
# Use environment variables for configuration
export FAKEKEY_PASSWORD="ci-secret"
fakekey add --service github --key "$GITHUB_TOKEN" --template
fakekey start
```

## Development

```bash
# Clone repository
git clone https://github.com/happyvibing/fakekey.git
cd fakekey

# Build
cargo build

# Run
cargo run -- init
cargo run -- start

# Format
cargo fmt

# Static check
cargo clippy
```

## License

Apache License 2.0

## Contributing

Issues and Pull Requests are welcome!
