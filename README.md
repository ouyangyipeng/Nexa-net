# Nexa-net

[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)]()

**Nexa-net** 是一个去中心化的 M2M (Machine-to-Machine) 通信基础设施，为 AI Agent 网络提供安全、高效、经济的服务发现与调用能力。

## 🌟 核心特性

- **去中心化身份 (Nexa-DID)** - 基于 W3C DID 规范的去中心化身份体系
- **语义服务发现** - 基于向量相似度的智能服务路由
- **流式 RPC** - 支持双向流式通信的高性能 RPC 框架
- **微交易经济** - Layer 2 状态通道支持高频微交易
- **非侵入式集成** - Sidecar Proxy 模式，无需修改现有应用

## 📐 架构概览

```
┌─────────────────────────────────────────────────────────────┐
│                      Nexa-net 四层架构                        │
├─────────────────────────────────────────────────────────────┤
│  Layer 1: Identity    │ Nexa-DID, mTLS, Verifiable Credentials │
│  Layer 2: Discovery   │ Capability Schema, Semantic Router, DHT │
│  Layer 3: Transport   │ Frame Protocol, Streaming RPC, LZ4/Zstd │
│  Layer 4: Economy     │ State Channel, Micro-Receipt, Token     │
└─────────────────────────────────────────────────────────────┘
```

## 🚀 快速开始

### 环境要求

- Rust 1.70+
- Cargo

### 编译

```bash
# 克隆仓库
git clone https://github.com/ouyangyipeng/Nexa-net.git
cd Nexa-net

# 编译
cargo build --release

# 运行测试
cargo test --lib
```

### 运行 Nexa-Proxy

```bash
# 运行代理服务
cargo run --bin nexa-proxy
```

代理服务将在以下端口启动：
- REST API: `http://127.0.0.1:7070/api/v1`
- gRPC: `127.0.0.1:7071`

## 📦 项目结构

```
src/
├── identity/           # Layer 1: 身份与零信任
│   ├── did.rs          # Nexa-DID 实现
│   ├── key_management.rs # Ed25519/X25519 密钥管理
│   └── credential.rs   # Verifiable Credentials
├── discovery/          # Layer 2: 语义发现
│   ├── capability.rs   # 能力注册表
│   ├── router.rs       # 多因子语义路由
│   ├── vectorizer.rs   # 语义向量化
│   └── embedding/      # Embedding 模块 (NEW)
│       ├── mod.rs      # Embedder trait
│       ├── mock.rs     # Mock Embedder
│       └── onnx.rs     # ONNX Runtime Embedder
├── transport/          # Layer 3: 传输协议
│   ├── frame.rs        # 12字节帧协议
│   ├── stream.rs       # 多路复用流
│   ├── rpc.rs          # 流式 RPC 引擎
│   └── negotiator.rs   # SYN-NEXA/ACK-SCHEMA 握手
├── economy/            # Layer 4: 经济层
│   ├── channel.rs      # 状态通道管理
│   ├── receipt.rs      # 微交易收据
│   └── budget.rs       # 预算控制器
├── storage/            # 持久化存储 (NEW)
│   ├── mod.rs          # Storage traits
│   └── memory.rs       # Memory Store
├── security/           # 安全模块 (NEW)
│   ├── audit.rs        # 审计日志
│   ├── key_rotation.rs # 密钥轮换
│   ├── rate_limit.rs   # 速率限制
│   └── secure_storage.rs # 加密存储
├── proxy/              # Nexa-Proxy 守护进程
│   ├── server.rs       # REST/gRPC 服务
│   └── config.rs       # 配置管理
└── api/                # SDK 接口
    └── sdk.rs          # NexaClient SDK
```

## 🔧 API 端点

| 端点 | 方法 | 描述 |
|------|------|------|
| `/api/v1/call` | POST | 发起网络调用 |
| `/api/v1/register` | POST | 注册能力 |
| `/api/v1/discover` | POST | 发现服务 |
| `/api/v1/channels` | GET | 列出通道 |
| `/api/v1/balance/:did` | GET | 查询余额 |
| `/api/v1/health` | GET | 健康检查 |

## 💻 SDK 使用

```rust
use nexa_net::api::sdk::{NexaClient, NexaClientBuilder, CallOptions};

// 创建客户端
let client = NexaClientBuilder::new()
    .endpoint("http://127.0.0.1:7070")
    .timeout_ms(30000)
    .budget(100)
    .build();

// 发起调用
let response = client.call(
    "translate English to Chinese",
    data,
    CallOptions::new().with_budget(50)
).await?;

// 发现服务
let routes = client.discover("translation service", 5).await?;
```

## 📊 帧协议格式

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                        Length (4 bytes)                       |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| Type (1 byte) |   Stream ID (4 bytes)         | Flags | Rsvd  |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                                                               |
+                       Payload (variable)                      +
|                                                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

## 🧪 测试

```bash
# 运行所有单元测试
cargo test --lib

# 运行集成测试
cargo test

# 运行基准测试
cargo bench

# 运行特定测试
cargo test test_rpc_header
```

当前测试状态: **130 个单元测试 + 15 个 embedding 测试 + 12 个集成测试全部通过**

## 🔧 Feature Flags

```toml
# Embedding 支持
cargo build --features embedding-onnx

# 存储支持
cargo build --features storage-postgres
cargo build --features storage-redis
cargo build --features storage-full
```

## 📚 文档

详细设计文档位于 [`docs/`](docs/) 目录：

- [用户指南](docs/USER_GUIDE.md) - 安装与使用指南 ⭐ NEW
- [架构设计](docs/ARCHITECTURE.md) - 系统整体架构
- [身份层](docs/IDENTITY_LAYER.md) - DID 与零信任
- [发现层](docs/DISCOVERY_LAYER.md) - 语义发现与路由
- [传输层](docs/TRANSPORT_LAYER.md) - 帧协议与 RPC
- [经济层](docs/ECONOMY_LAYER.md) - 状态通道与微交易
- [API 参考](docs/API_REFERENCE.md) - 完整 API 文档
- [安全设计](docs/SECURITY.md) - 安全架构与最佳实践

## 🤝 贡献

欢迎贡献！请查看 [贡献指南](CONTRIBUTING.md)。

## 📄 许可证

本项目采用 MIT 许可证 - 详见 [LICENSE](LICENSE) 文件。

## 🙏 致谢

- [ed25519-dalek](https://github.com/dalek-cryptography/ed25519-dalek) - Ed25519 签名
- [x25519-dalek](https://github.com/dalek-cryptography/x25519-dalek) - X25519 密钥协商
- [lz4_flex](https://github.com/PSeitz/lz4_flex) - LZ4 压缩
- [axum](https://github.com/tokio-rs/axum) - Web 框架

---

**Nexa-net** - 为 AI Agent 网络构建的去中心化通信基础设施