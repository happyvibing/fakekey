# FakeKey - API Key Proxy Agent

In the era of AI Agents like Openclaw, ClaudeCode, etc., we have to expose various service API tokens directly in environment variables. Your api_key will be inserted into context and known by model service providers, known by the lobsters you trust, perhaps captured and read by some skill, and more likely to be directly known when strangers ask your lobster. With too many leakage cases, I cannot trust to expose my credit card-bound api_key directly to any Agent and local environment variables. Thus, FakeKey was born - the safest measure is to never expose the real api_key.

FakeKey is a high-performance API key proxy program developed in Rust. Through intelligent proxy technology, it can automatically replace fake keys with real keys in any network request without exposing real credentials, while maintaining complete HTTP API compatibility and performance.

## How It Works

```
┌─────────────────┐         ┌──────────────────────────┐         ┌─────────────────┐
│   Client Agent  │ HTTP/S  │   FakeKey Proxy          │ HTTP/S  │  External API   │
│                 │────────▶│  1. TLS Decryption        │────────▶ │                 │
│  Uses fake key   │         │  2. Identify and replace    │         │  Receives real key│
│  sk-xxx_fk      │         │  3. Forward request         │         │  sk-xxx         │
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

### One-Click Initialization

```bash
fakekey onboard
```

During the process, you will be prompted to trust the CA certificate. For first-time use, you need to add the CA certificate to the system trust list:

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

# Generate OpenAI type fake key
fakekey add --name my-openai-key --key "sk-proj-xxxxx" --template openai

# Generate fake key with custom header
fakekey add --name my-custom --key "xxxxx" --header "X-Custom-Key"

# View available templates
fakekey templates

# List all configured keys
fakekey list

# View specific key configuration
fakekey show --name my-openai-key

# Delete key configuration
fakekey remove --name my-openai-key

# View proxy status
fakekey status

# Run in foreground
fakekey start

# Run in background (daemon mode)
fakekey start --daemon

# Stop proxy
fakekey stop

# View logs
fakekey logs
```

### Setting Up Proxy in Agent or Application

- Replace the real API KEY with the generated fake API KEY in your Agent or application
- Set the network proxy to `http://127.0.0.1:1155` in your Agent or application. EG: `export http_proxy=http://127.0.0.1:1155` `export https_proxy=http://127.0.0.1:1155`


## Security

1. **Key Protection** - Real keys are stored locally only, configuration files are automatically encrypted using CA private key (JSON format)
2. **Certificate Security** - Locally generated CA certificate, private key file permissions 0600, also used for configuration encryption
3. **Network Security** - Only listens on local 127.0.0.1, supports host whitelist
4. **Log Desensitization** - Automatically hides sensitive information
5. **Audit Trail** - All critical operations are recorded to audit logs


## License

Apache License 2.0

## Contributing

Issues and Pull Requests are welcome!
