# FakeKey - API Key Proxy Agent

In the era of popular AI Agents like Openclaw, ClaudeCode, etc., we have to expose various service API Tokens directly in environment variables. Your api_key gets stuffed into context and known by model service providers, known by the lobsters you trust, perhaps captured and read by some skill, and even more likely to be directly known when strangers ask your claw. There are too many leakage cases, I cannot trust to expose my credit card-bound api_key directly to any Agent and local environment variables, so FakeKey was born, the safest measure is to never expose the real api_key.

FakeKey is a high-performance API key proxy program developed in Rust. Through intelligent proxy technology, it can automatically replace fake keys with real keys in any network request, allowing your application code to avoid exposing real credentials while maintaining complete HTTP API compatibility and performance.

## How It Works

```
┌─────────────────┐         ┌──────────────────────────┐         ┌─────────────────┐
│   Client App    │ HTTP/S  │   FakeKey Proxy          │ HTTP/S  │  External API   │
│                 │────────▶│  1. TLS Decryption        │────────▶ │                 │
│  Uses fake key  │         │  2. Identify and replace   │         │  Receives real  │
│  sk-xxx_fk      │         │     fake key               │         │  key sk-xxx     │
│                 │         │  3. Forward request        │         │                 │
└─────────────────┘         └──────────────────────────┘         └─────────────────┘
```


## Quick Start

### Installation

```bash
# Build from source
cargo build --release

# Install to system
cargo install --path .
```

### Initialize

```bash
# Initialize configuration and CA certificate
fakekey init
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
# Foreground run
fakekey start

# Background run (daemon mode)
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

## Security

1. **Key Protection** - Real keys are stored locally only, with optional encrypted storage
2. **Certificate Security** - Locally generated CA certificates, private key file permissions 0600
3. **Network Security** - Only listens on local 127.0.0.1, supports host whitelist
4. **Log Desensitization** - Automatically hides sensitive information
5. **Audit Trail** - All critical operations are logged to audit logs


## License

Apache License 2.0

## Contributing

Issues and Pull Requests are welcome!
