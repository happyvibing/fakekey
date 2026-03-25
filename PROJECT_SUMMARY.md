# FakeKey Project Summary

## 项目概述

FakeKey 是一个基于 Rust 开发的高性能 API 密钥代理程序，旨在解决 AI Agent 时代 API 密钥泄露的问题。通过智能代理技术，它能够在网络请求中自动将假密钥替换为真实密钥，无需暴露真实凭证。

## 核心功能

### 🔐 安全特性
- **密钥替换**: 自动识别并替换 HTTP 请求中的假密钥
- **TLS 代理**: 完整的 HTTPS 请求代理支持
- **证书管理**: 自动生成和管理 CA 证书
- **加密存储**: 真实密钥使用 CA 私钥加密存储

### 🚀 易用性
- **交互式设置**: `fakekey onboard` 一键配置向导
- **多平台支持**: Linux, macOS, Windows
- **多种安装方式**: Cargo, Homebrew, 二进制下载
- **后台运行**: Daemon 模式支持

### 📊 监控与审计
- **详细日志**: 完整的请求和响应日志
- **审计跟踪**: 密钥使用记录
- **状态检查**: 实时代理状态监控
- **端口验证**: 确保代理真正在监听

## 技术架构

### 核心组件
1. **代理服务器** (`proxy.rs`): HTTP/HTTPS 请求处理
2. **密钥处理器** (`key_handler.rs`): 密钥识别和替换
3. **证书管理** (`cert.rs`): TLS 证书生成和管理
4. **配置管理** (`config.rs`): 设置和密钥存储
5. **审计日志** (`audit.rs`): 安全事件记录

### 依赖技术
- **异步运行时**: Tokio
- **HTTP 库**: Hyper + hyper-util
- **TLS**: rustls + tokio-rustls
- **CLI**: clap
- **序列化**: serde + serde_json
- **日志**: tracing + tracing-subscriber

## 发布状态

### ✅ 已完成
- **v0.1.0** - 初始版本发布
- **v0.1.1** - 改进状态检查，添加二进制文件

### 📦 发布渠道
- **crates.io**: `cargo install fakekey`
- **GitHub Releases**: 预编译二进制文件
- **安装脚本**: 一键安装
- **Homebrew**: `brew install happyvibing/tap/fakekey`

### 🔄 自动化流程
- **GitHub Actions**: 自动构建、测试、发布
- **多平台构建**: Linux, macOS, Windows (x86_64 + ARM64)
- **自动发布**: 标签推送触发完整发布流程

## 使用场景

### 🤖 AI Agent 开发
- 保护 OpenAI API 密钥
- 保护 Anthropic API 密钥
- 保护其他 AI 服务 API 密钥

### 🔧 开发环境
- 本地开发时的密钥保护
- CI/CD 流水线中的密钥管理
- 团队协作时的密钥共享

### 🏢 企业应用
- API 密钥的集中管理
- 审计和合规要求
- 安全开发流程

## 安装和使用

### 快速安装
```bash
# 方法 1: Cargo (推荐)
cargo install fakekey

# 方法 2: 安装脚本
curl -fsSL https://raw.githubusercontent.com/happyvibing/fakekey/main/install.sh | bash

# 方法 3: Homebrew
brew install happyvibing/tap/fakekey
```

### 基本使用
```bash
# 交互式设置
fakekey onboard

# 添加 API 密钥
fakekey add --name my-openai-key --key "sk-..." --template openai

# 启动代理
fakekey start --daemon

# 检查状态
fakekey status

# 查看日志
fakekey logs
```

## 项目结构

```
fakekey/
├── src/
│   ├── main.rs          # CLI 入口点
│   ├── cli.rs           # 命令行定义
│   ├── proxy.rs         # 代理服务器
│   ├── key_handler.rs   # 密钥处理逻辑
│   ├── cert.rs          # 证书管理
│   ├── config.rs        # 配置管理
│   ├── audit.rs         # 审计日志
│   ├── daemon.rs        # 后台运行
│   ├── security.rs      # 安全工具
│   └── templates.rs     # 服务模板
├── examples/
│   └── quick_start.sh   # 快速开始示例
├── homebrew/
│   └── fakekey.rb       # Homebrew formula
├── .github/workflows/
│   └── release.yml      # 自动发布流程
├── install.sh           # 安装脚本
├── README.md            # 英文文档
├── README_CN.md         # 中文文档
├── RELEASE_CHECKLIST.md # 发布检查清单
└── PROJECT_SUMMARY.md   # 项目总结
```

## 贡献指南

### 开发环境设置
```bash
git clone https://github.com/happyvibing/fakekey.git
cd fakekey
cargo build
cargo test
```

### 发布流程
1. 更新版本号
2. 提交更改
3. 创建标签
4. 推送到 GitHub
5. 自动发布流程触发

详细的发布流程请参考 `RELEASE_CHECKLIST.md`。

## 未来计划

### 短期目标
- [ ] 添加更多服务模板
- [ ] 改进错误处理
- [ ] 添加性能监控
- [ ] 支持更多密钥位置

### 长期目标
- [ ] Web 管理界面
- [ ] 集群部署支持
- [ ] 插件系统
- [ ] 企业版功能

## 许可证

本项目采用 Apache-2.0 许可证，详情请参考 LICENSE 文件。

## 联系方式

- **GitHub**: https://github.com/happyvibing/fakekey
- **Issues**: https://github.com/happyvibing/fakekey/issues
- **crates.io**: https://crates.io/crates/fakekey

---

*最后更新: 2026-03-26*
