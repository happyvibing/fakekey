# FakeKey - API Key Proxy Agent

FakeKey 是一个基于 Rust 开发的 CLI 代理程序，用于管理和替换 API 密钥。通过设置网络代理，应用程序可以使用假密钥，FakeKey 会在请求中自动替换为真实密钥，从而保护敏感凭证。

## 核心特性

### ✅ 已实现功能

- **HTTP/HTTPS 代理** - 支持 MITM 方式解密 HTTPS 流量
- **密钥管理** - 添加、列出、查看、删除 API 密钥配置
- **假密钥生成** - 自动生成与原密钥相同长度的假密钥（后缀 `_fk`）
- **密钥替换** - 在 Header、URL 参数、请求体中自动替换假密钥为真实密钥
- **证书管理** - 自动生成和管理 CA 证书及服务器证书
- **配置加密** - 使用 AES-256-GCM 加密保护配置文件
- **日志脱敏** - 自动在日志中隐藏敏感信息
- **审计日志** - 记录所有关键操作到审计日志
- **服务模板** - 预置 OpenAI、GitHub、Anthropic 等常用服务配置
- **Daemon 模式** - 支持后台运行
- **完整测试** - 包含单元测试和集成测试

## 快速开始

### 安装

```bash
# 从源码编译
cargo build --release

# 安装到系统
cargo install --path .
```

### 初始化

```bash
# 初始化配置和 CA 证书
fakekey init

# 输出:
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

### 添加 API 密钥

```bash
# 使用预设模板添加 OpenAI 密钥
fakekey add --service openai --key "sk-proj-xxxxx" --template

# 输出:
# Using template: OpenAI API (api.openai.com)
# Added API key for service: openai
# Fake key: sk-proj-xxxxx_fk
```

### 查看可用模板

```bash
fakekey templates

# 输出:
# SERVICE         KEY PATTERN          DESCRIPTION
# -------------------------------------------------------------------------------
# openai          sk-                  OpenAI API (api.openai.com)
# anthropic       sk-ant-              Anthropic Claude API (api.anthropic.com)
# github          ghp_                 GitHub Personal Access Token (api.github.com)
# google          AIza                 Google Cloud API (googleapis.com)
# huggingface     hf_                  Hugging Face API (huggingface.co)
# deepseek        sk-                  DeepSeek API (api.deepseek.com)
```

### 启动代理

```bash
# 前台运行
fakekey start

# 后台运行（daemon 模式）
fakekey start --daemon

# 指定端口
fakekey start --port 8080
```

### 信任 CA 证书

首次使用时需要将 CA 证书添加到系统信任列表：

```bash
# 导出 CA 证书
fakekey cert export

# macOS
sudo security add-trusted-cert -d -r trustRoot \
  -k /Library/Keychains/System.keychain ~/.fakekey/certs/ca.crt

# Linux
sudo cp ~/.fakekey/certs/ca.crt /usr/local/share/ca-certificates/fakekey.crt
sudo update-ca-certificates
```

### 配置应用程序

在你的应用中设置：
- **API Key**: 使用假密钥 `sk-proj-xxxxx_fk`
- **Proxy**: `http://127.0.0.1:1157`

## CLI 命令

### 基本命令

```bash
# 列出所有配置的密钥
fakekey list

# 查看特定服务配置
fakekey show --service openai

# 删除密钥配置
fakekey remove --service openai

# 查看代理状态
fakekey status

# 停止代理
fakekey stop

# 查看日志
fakekey logs
```

### 配置加密

```bash
# 启用配置加密
export FAKEKEY_PASSWORD="your-secure-password"
fakekey encrypt --enable

# 禁用配置加密
fakekey encrypt --disable
```

## 配置文件示例

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

## 架构设计

```
┌─────────────────┐         ┌──────────────────────────┐         ┌─────────────────┐
│   Client App    │ HTTPS   │   FakeKey Proxy          │ HTTPS   │  External API   │
│                 │────────▶│  1. TLS 解密             │────────▶│                 │
│  使用假密钥      │         │  2. 识别并替换假密钥      │         │  接收真实密钥    │
│  sk-xxx_fk      │         │  3. 转发请求             │         │  sk-xxx         │
└─────────────────┘         └──────────────────────────┘         └─────────────────┘
```

## 模块说明

- **config** - 配置管理和假密钥生成
- **proxy** - HTTP/HTTPS 代理服务器
- **cert** - CA 证书和服务器证书管理
- **key_handler** - 密钥识别和替换逻辑
- **security** - 配置加密和数据脱敏
- **audit** - 审计日志记录
- **templates** - 预设服务模板
- **daemon** - 后台进程管理

## 测试

```bash
# 运行所有测试
cargo test

# 运行特定测试
cargo test test_key_replacement

# 查看测试覆盖率
cargo test --verbose
```

测试包括：
- 17 个单元测试
- 6 个集成测试
- 覆盖密钥替换、配置管理、加密、模板等核心功能

## 安全考虑

1. **密钥保护** - 真实密钥仅存储在本地，可选加密存储
2. **证书安全** - 本地生成 CA 证书，私钥文件权限 0600
3. **网络安全** - 仅监听本地 127.0.0.1，支持主机白名单
4. **日志脱敏** - 自动隐藏敏感信息
5. **审计追踪** - 所有关键操作记录到审计日志

## 使用场景

### IDE 集成开发

```bash
# 配置 FakeKey
fakekey add --service openai --key "sk-real-key" --template
fakekey start --daemon

# 在 IDE 中配置
# API Key: sk-real-key_fk
# Proxy: http://127.0.0.1:1157
```

### CI/CD 环境

```bash
# 使用环境变量配置
export FAKEKEY_PASSWORD="ci-secret"
fakekey add --service github --key "$GITHUB_TOKEN" --template
fakekey start
```

## 开发

```bash
# 克隆仓库
git clone https://github.com/happyvibing/fakekey.git
cd fakekey

# 构建
cargo build

# 运行
cargo run -- init
cargo run -- start

# 格式化
cargo fmt

# 静态检查
cargo clippy
```

## 许可证

Apache License 2.0

## 贡献

欢迎提交 Issue 和 Pull Request！
