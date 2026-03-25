# FakeKey - API Key Proxy Agent

在 Openclaw, ClaudeCode 等 AI Agent 盛行的当下，我们不得不将各种服务的 API Token 直接暴露在环境变量中。你的 api_key 会被塞入上下文被模型服务商知道，会被你所信任的龙虾知道，也许是被某个skill捕获读取，更有可能陌生人询问你的龙虾直接得知。太多泄露的案例，我无法信任的将自己绑定信用卡的 api_key 直接暴露给任何 Agent 和本地环境变量中，于是乎 FakeKey 应运而生，最安全的措施就是永远不暴露真实的 api_key。


FakeKey 是基于 Rust 开发的高性能 API 密钥代理程序。通过智能代理技术，它能够在任何网络请求中自动将假密钥替换为真实密钥，无需暴露真实凭证，同时保持完整的 HTTP API 兼容性和性能。

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

```bash
# 从源码编译
cargo build --release

# 安装到系统
cargo install --path .
```

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

# 生成自定义 Header 的密钥假 KEY
fakekey add --name my-custom --key "xxxxx" --header "X-Custom-Key"

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

# 前台运行
fakekey start

# 后台运行（daemon 模式）
fakekey start --daemon

# 停止代理
fakekey stop

# 查看日志
fakekey logs
```

### 在Agent或应用中设置代理

- 将生成的假 API KEY 代替真的应用到 Agent 或应用中
- 在 Agent 或应用中设置网络代理为 `http://127.0.0.1:1155` EG: `export http_proxy=http://127.0.0.1:1155` `export https_proxy=http://127.0.0.1:1155`


## 安全

1. **密钥保护** - 真实密钥仅存储在本地，配置文件使用 CA 私钥自动加密（JSON 格式）
2. **证书安全** - 本地生成 CA 证书，私钥文件权限 0600，同时用于配置加密
3. **网络安全** - 仅监听本地 127.0.0.1，支持主机白名单
4. **日志脱敏** - 自动隐藏敏感信息
5. **审计追踪** - 所有关键操作记录到审计日志


## 许可证

Apache License 2.0

## 贡献

欢迎提交 Issue 和 Pull Request！
