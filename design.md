# FakeKey - API Key Proxy Agent Design

## 项目概述

FakeKey 是一个基于 Rust 开发的 CLI 代理程序，用于管理和替换 API 密钥。其他程序可以通过设置网络代理来使用此服务，实现 API 密钥的安全代理和替换。

## 核心功能

### 1. API 密钥管理
- **配置管理**: 用户配置真实的 API 密钥（OpenAI、Google、GitHub 等）
- **假密钥生成**: 自动生成与原密钥相同长度的可识别假密钥
- **密钥映射**: 维护假密钥到真实密钥的映射关系

### 2. 网络代理
- **HTTP/HTTPS 代理**: 监听指定端口（默认 1155），代理网络流量
- **HTTPS 解密**: 使用 MITM（中间人）技术，生成自签名证书解密 HTTPS 流量
- **流量拦截**: 拦截 HTTP/HTTPS 请求的 Header、URL 参数、请求体中的 API 密钥
- **密钥替换**: 将假密钥替换为真实密钥后转发到目标服务器

### 3. 安全特性
- **本地存储**: 真实密钥仅存储在本地
- **加密存储**: 配置文件加密保护
- **访问控制**: 可选的访问控制机制

## 技术架构

### 核心组件

```
┌─────────────────┐         ┌──────────────────────────┐         ┌─────────────────┐
│   Client App    │ HTTPS   │   FakeKey Proxy          │ HTTPS   │  External API   │
│                 │────────▶│  1. TLS 解密             │────────▶│                 │
│  使用假密钥      │         │  2. 识别并替换假密钥      │         │  接收真实密钥    │
│  sk-xxx_fk      │         │  3. 转发请求             │         │  sk-xxx         │
└─────────────────┘         └──────────────────────────┘         └─────────────────┘
```

### 模块设计

#### 1. 配置模块 (config)
- API 密钥配置管理
- 假密钥生成算法
- 配置文件读写

#### 2. 代理模块 (proxy)
- HTTP/HTTPS 代理服务器（支持 CONNECT 方法）
- TLS 中间人处理（动态证书生成）
- 连接管理和路由
- 流量拦截和解析

#### 3. 密钥处理模块 (key_handler)
- 密钥识别和匹配（支持正则表达式）
- 多位置密钥扫描（Header、URL、Body）
- 密钥替换逻辑
- 请求/响应处理

#### 4. 安全模块 (security)
- 配置加密/解密
- 证书生成和管理（CA 证书、服务器证书）
- 访问验证
- 日志脱敏

#### 5. 证书模块 (cert)
- 根 CA 证书生成（~/.fakekey/certs/ca/）
- 动态服务器证书签发
- 证书缓存管理（~/.fakekey/certs/cache/）

## 数据结构

### API 密钥配置
```rust
struct ApiKeyConfig {
    service: String,        // "openai", "google", "github"
    real_key: String,       // 真实 API 密钥
    fake_key: String,       // 生成的假密钥（后缀 _fk）
    header_name: String,    // Header 名称，如 "Authorization"
    scan_locations: Vec<ScanLocation>,  // 扫描位置：Header、URL、Body
    created_at: DateTime<Utc>,
}

enum ScanLocation {
    Header(String),         // Header 名称
    UrlParam(String),       // URL 参数名
    JsonBody(String),       // JSON Body 字段路径
}

struct ProxyConfig {
    listen_port: u16,       // 默认 1155
    allowed_hosts: Vec<String>,
    log_level: LogLevel,
    encryption_enabled: bool,
    data_dir: String,       // 数据目录路径 ~/.fakekey/
}
```

### 假密钥生成策略
- **长度保持**: 与原密钥长度相同
- **唯一性**: 确保每个假密钥唯一
- **格式兼容**: 保持原密钥的字符集格式
- **可识别性**: 使用特定后缀（如 `_fk`）便于识别

## 工作流程

### 1. 初始化流程
1. 用户启动 FakeKey 应用
2. 加载或创建配置文件
3. 生成假密钥映射
4. 启动代理服务器

### 2. 代理处理流程

**HTTP 请求:**
1. 接收客户端 HTTP 请求
2. 扫描 Header、URL、Body 中的假密钥
3. 查找映射表，替换为真实密钥
4. 转发请求到目标服务器
5. 返回响应给客户端

**HTTPS 请求（MITM）:**
1. 接收客户端 CONNECT 请求
2. 为目标域名动态生成服务器证书（使用本地 CA 签名）
3. 与客户端建立 TLS 连接
4. 解密 HTTPS 流量
5. 扫描并替换假密钥
6. 与目标服务器建立新的 TLS 连接
7. 转发请求，返回响应

### 3. 密钥管理流程
1. 用户添加新的 API 密钥配置
2. 生成对应的假密钥
3. 更新映射关系
4. 可选：重启代理服务

## CLI 接口设计

### 基本命令
```bash
# 初始化配置（生成 CA 证书）
fakekey init

# 启动代理服务
fakekey start --port 1155 --daemon

# 添加 API 密钥
fakekey add --service openai --key "sk-proj-xxxxx"
# 自动生成假密钥并显示：sk-proj-xxxxx_fk

# 列出所有配置的密钥
fakekey list

# 查看特定服务的配置
fakekey show --service openai

# 删除配置
fakekey remove --service openai

# 查看代理状态和统计
fakekey status

# 查看实时日志
fakekey logs --follow

# 导出 CA 证书（用于系统信任）
fakekey cert export --output ~/.fakekey/certs/ca.crt

# 停止服务
fakekey stop
```

### 配置文件示例
```yaml
# ~/.fakekey/config.yaml
proxy:
  port: 1155
  log_level: info
  data_dir: "~/.fakekey"
  allowed_hosts: ["api.openai.com", "googleapis.com", "api.github.com"]

api_keys:
  - service: "openai"
    real_key: "sk-proj-1234567890abcdefghijk"
    fake_key: "sk-proj-1234567890abcdefg_fk"  # 后缀 _fk
    header_name: "Authorization"
    scan_locations:
      - type: "header"
        name: "Authorization"
        
  - service: "github"
    real_key: "ghp_1234567890abcdefghijklmnopqrst"
    fake_key: "ghp_1234567890abcdefghijklmnop_fk"
    header_name: "Authorization"
    scan_locations:
      - type: "header"
        name: "Authorization"
      - type: "url_param"
        name: "token"

security:
  encrypt_config: true
```

## 技术选型

### 核心依赖
- **tokio**: 异步运行时
- **hyper**: HTTP 服务器和客户端
- **hyper-rustls**: HTTPS 支持
- **rustls**: TLS 库
- **rcgen**: 动态证书生成
- **serde** / **serde_yaml**: 配置序列化
- **clap**: CLI 参数解析
- **regex**: 密钥模式匹配
- **tracing** / **tracing-subscriber**: 结构化日志
- **ring** / **aes-gcm**: 配置文件加密

## 安全考虑

### 1. 密钥保护
- 真实密钥内存加密
- 配置文件 AES-256-GCM 加密存储
- 日志中自动脱敏（隐藏真实密钥）
- 密钥仅在替换时短暂解密

### 2. 证书安全
- 本地生成 CA 根证书（~/.fakekey/certs/ca/）
- 用户需手动信任 CA 证书
- 证书私钥安全存储（~/.fakekey/certs/ca/key.pem，0600 权限）
- 动态生成的服务器证书缓存和过期管理（~/.fakekey/certs/cache/）

### 3. 网络安全
- HTTPS 证书验证（验证目标服务器证书）
- 仅代理配置的允许主机
- 本地监听（127.0.0.1）防止外部访问

### 4. 运行安全
- 最小权限原则（~/.fakekey/ 目录权限 0700）
- 审计日志记录（~/.fakekey/logs/audit.log）
- 异常检测和告警

## 实现优先级

### MVP（最小可行产品）
- [x] 项目架构设计
- [x] HTTP 代理基础功能
- [x] HTTPS MITM 和证书生成
- [x] 假密钥生成（后缀策略）
- [x] Header 中密钥替换
- [x] URL 参数中密钥替换
- [x] Body 中密钥替换
- [x] CLI 命令（init, start, add, list, stop）
- [x] 配置文件管理
- [x] 证书缓存管理
- [x] 审计日志系统
- [x] 配置文件加密
- [x] 日志文件输出
- [x] 服务模板系统
- [x] 后台运行模式

### 已完成的高级功能
- [x] 动态证书生成和缓存
- [x] 多位置密钥扫描（Header、URL、Body）
- [x] 安全的密钥存储和加密
- [x] 审计日志记录
- [x] 日志脱敏处理
- [x] CA 证书管理
- [x] 进程管理（PID 文件）
- [x] 允许主机白名单
- [x] 服务模板（OpenAI, GitHub, Google, Anthropic, HuggingFace, DeepSeek）

### 待完成功能（可选增强）
- [ ] 实时日志查看（follow模式）
- [ ] WebSocket 代理支持
- [ ] HTTP/2 支持
- [ ] 性能监控和统计
- [ ] 配置导入/导出（团队共享）
- [ ] GUI 管理界面
- [ ] 更多服务模板
- [ ] 密钥轮换策略
- [ ] 多用户支持
- [ ] API 使用统计


## 测试策略

### 单元测试
- 密钥生成算法测试
- 配置管理测试
- 密钥替换逻辑测试

### 集成测试
- 代理功能测试
- 端到端 API 调用测试
- 多服务并发测试

### 安全测试
- 密钥泄露检测
- 配置文件安全测试
- 网络攻击防护测试

## 使用场景示例

### 场景 1: 保护 IDE 中的 OpenAI API Key
```bash
# 1. 初始化并添加真实密钥
fakekey init
# 自动创建 ~/.fakekey/ 目录结构和 CA 证书

fakekey add --service openai --key "sk-proj-real-key-123456789"
# 输出: Generated fake key: sk-proj-real-key-123456_fk

# 2. 启动代理
fakekey start --daemon

# 3. 在 IDE/应用中配置
# - API Key: sk-proj-real-key-123456_fk
# - Proxy: http://127.0.0.1:1155

# 4. 信任 CA 证书（仅首次）
# macOS: sudo security add-trusted-cert -d -r trustRoot -k /Library/Keychains/System.keychain ~/.fakekey/certs/ca.crt
# Linux: sudo cp ~/.fakekey/certs/ca.crt /usr/local/share/ca-certificates/fakekey.crt && sudo update-ca-certificates
```

### 场景 2: 团队共享配置（不包含真实密钥）
```bash
# 开发者 A: 导出配置模板（仅包含假密钥）
fakekey export --template --output team-config.yaml
# 导出文件不包含真实密钥，仅包含假密钥和服务配置

# 开发者 B: 使用模板，填入自己的真实密钥
fakekey import --template team-config.yaml
# 然后手动添加真实密钥到 ~/.fakekey/config.yaml
```

## 部署和分发

### 打包方式
- 单一可执行文件（静态链接）
- 跨平台支持（Linux、macOS、Windows）

### 安装方式
```bash
# Cargo 安装
cargo install fakekey

# 二进制下载
curl -sSL https://github.com/happyvibing/fakekey/releases/latest/download/fakekey-macos -o fakekey
chmod +x fakekey

# 首次使用
fakekey init
# 创建 ~/.fakekey/ 目录结构：
# ~/.fakekey/
#   ├── config.yaml      # 主配置文件（加密）
#   ├── certs/           # 证书目录
#   │   ├── ca/          # CA 根证书
#   │   │   ├── cert.pem # CA 根证书
#   │   │   └── key.pem  # CA 私钥（0600 权限）
#   │   ├── cache/       # 动态生成证书缓存
#   │   └── ca.crt       # 导出用的 CA 证书副本
#   ├── logs/            # 日志文件
#   │   ├── proxy.log    # 代理日志
#   │   └── audit.log    # 审计日志
#   └── pid              # 进程 ID 文件
```
