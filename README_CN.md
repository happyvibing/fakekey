# FakeKey - API Key Proxy Agent

在 Openclaw, ClaudeCode 等 AI Agent 盛行的当下，我们不得不将各种服务的 API Token 直接暴露在环境变量中。你的 api_key 会被塞入上下文被模型服务商知道，会被你所信任的龙虾知道，也许是被某个skill捕获读取，更有可能陌生人询问你的claw直接得知。太多泄露的案例，我无法信任的将自己绑定信用卡的 api_key 直接暴露给任何 Agent 和本地环境变量中，于是乎 FakeKey 应运而生，最安全的措施就是永远不暴露真实的 api_key。


FakeKey 是基于 Rust 开发的高性能 API 密钥代理程序。通过智能代理技术，它能够在任何网络请求中自动将假密钥替换为真实密钥，让您的应用代码无需暴露真实凭证，同时保持完整的 HTTP API 兼容性和性能。

## 如何工作

```
┌─────────────────┐         ┌──────────────────────────┐         ┌─────────────────┐
│   Client App    │ HTTP/S  │   FakeKey Proxy          │ HTTP/S  │  External API   │
│                 │────────▶│  1. TLS 解密             │────────▶ │                 │
│  使用假密钥       │         │  2. 识别并替换假密钥       │         │  接收真实密钥    │
│  sk-xxx_fk      │         │  3. 转发请求              │         │  sk-xxx         │
└─────────────────┘         └──────────────────────────┘         └─────────────────┘
```


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

## 安全

1. **密钥保护** - 真实密钥仅存储在本地，可选加密存储
2. **证书安全** - 本地生成 CA 证书，私钥文件权限 0600
3. **网络安全** - 仅监听本地 127.0.0.1，支持主机白名单
4. **日志脱敏** - 自动隐藏敏感信息
5. **审计追踪** - 所有关键操作记录到审计日志


## 许可证

Apache License 2.0

## 贡献

欢迎提交 Issue 和 Pull Request！
