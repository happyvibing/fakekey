# FakeKey - API Key Proxy Agent

在 Openclaw, ClaudeCode 等 AI Agent 盛行的当下，我们不得不将各种服务的 API Token 直接暴露在环境变量中。你的 api_key 会被塞入上下文被模型服务商知道，会被你所信任的龙虾知道，也许是被某个skill捕获读取，更有可能陌生人询问你的龙虾直接得知。太多泄露的案例，我无法信任的将自己绑定信用卡的 api_key 直接暴露给任何 Agent 和本地环境变量中，于是乎 FakeKey 应运而生，最安全的措施就是永远不暴露真实的 api_key。


FakeKey 是基于 Rust 开发的高性能 API 密钥代理程序。通过智能代理技术，它能够在 HTTP 请求头和 URL 中自动将假密钥替换为真实密钥，无需暴露真实凭证，同时保持完整的 HTTP API 兼容性和性能。

## 如何工作

```
┌─────────────────┐         ┌──────────────────────────┐         ┌─────────────────┐
│   Client Agent  │ HTTP/S  │   FakeKey Proxy          │ HTTP/S  │  External API   │
│                 │────────▶│  1. TLS 解密             │────────▶ │                 │
│  使用假密钥       │         │  2. 识别并替换假密钥       │         │  接收真实密钥    │
│  sk-xxx_fk      │         │  3. 转发请求              │         │  sk-xxx         │
└─────────────────┘         └──────────────────────────┘         └─────────────────┘
```


## 快速开始

### 安装

#### 快速安装（macOS / Linux）

```bash
curl -fsSL https://raw.githubusercontent.com/happyvibing/fakekey/main/install.sh | bash
```

#### Homebrew（macOS / Linux）

```bash
brew install happyvibing/tap/fakekey
```

#### Cargo（从 crates.io）

```bash
cargo install fakekey
```

#### 从源码编译安装

```bash
git clone https://github.com/happyvibing/fakekey.git
cd fakekey
cargo build --release
cargo install --path .
```

#### 下载预编译二进制

所有平台的预编译二进制文件可在 [GitHub Releases](https://github.com/happyvibing/fakekey/releases) 页面下载：

| 平台 | 文件 |
|------|------|
| macOS (Apple Silicon) | `fakekey-macos-arm64.tar.gz` |
| macOS (Intel) | `fakekey-macos-amd64.tar.gz` |
| Linux (x86_64) | `fakekey-linux-amd64.tar.gz` |
| Linux (ARM64) | `fakekey-linux-arm64.tar.gz` |
| Windows (x86_64) | `fakekey-windows-amd64.zip` |

### 一键初始化

```bash
fakekey onboard
```

过程中会提示你信任 CA 证书，首次使用时需要将 CA 证书添加到系统信任列表：

```bash
# macOS
sudo security add-trusted-cert -d -r trustRoot \
  -k /Library/Keychains/System.keychain ~/.fakekey/certs/ca.crt

# Linux
sudo cp ~/.fakekey/certs/ca.crt /usr/local/share/ca-certificates/fakekey.crt
sudo update-ca-certificates
```

### 基本命令

```bash
# 生成 OpenAI 类型密钥假 KEY
fakekey add --name my-openai-key --key "sk-proj-xxxxx" --template openai

# 生成自定义密钥假 KEY
fakekey add --name my-custom --key "xxxxx"

# 查看可用模板
fakekey templates

# 列出所有配置的密钥
fakekey list

# 查看特定密钥配置
fakekey show --name my-openai-key

# 删除密钥配置
fakekey remove --name my-openai-key

# 查看代理状态
fakekey status

# 运行（默认后台运行）
fakekey start

# 停止
fakekey stop

# 查看日志
fakekey logs
```

### 一键启动工具（推荐）

FakeKey 提供了便捷的方式来启动 CLI 工具，并自动配置代理保护：

```bash
# 启动 Claude Code 并自动配置代理保护
fakekey run claude

# 启动 OpenClaw 并自动配置代理保护
fakekey run openclaw

# 传递额外参数给工具
fakekey run claude --help
```

该命令会自动完成以下操作：
1. 检查代理是否运行，如果未运行则自动启动
2. 设置所有必要的环境变量（HTTP_PROXY、HTTPS_PROXY、NODE_EXTRA_CA_CERTS 等）
3. 启动工具并启用代理保护
4. 您的所有 API 密钥都将自动受到保护！

### 手动代理配置

如果您更喜欢手动配置：

- 将生成的假 API KEY 代替真的应用到 Agent 或应用中
- 在 Agent 或应用中设置网络代理为 `http://127.0.0.1:1155`

例如先设置网络代理:
```bash
export http_proxy=http://127.0.0.1:1155
export https_proxy=http://127.0.0.1:1155
export NODE_EXTRA_CA_CERTS=~/.fakekey/certs/ca.crt
```
然后再启动 Agent 如 `claude`、`openclaw`、`pi`

## 安全

1. **密钥保护** - 真实 API 密钥使用 AES-256-GCM 加密后存储在本地配置文件中；加密密钥安全存储于操作系统级密钥库（macOS Keychain / Linux Secret Service / Windows Credential Manager）
2. **证书安全** - 本地生成 CA 证书，私钥文件权限 0600，用于 TLS MITM 代理
3. **网络安全** - 仅监听本地 127.0.0.1，支持主机白名单
4. **日志脱敏** - 自动隐藏敏感信息
5. **审计追踪** - 所有关键操作记录到审计日志


## 许可证

Apache License 2.0

## 贡献

欢迎提交 Issue 和 Pull Request！
