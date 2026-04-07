<div align="center">
  <img src="docs/img/nexa-net-logo.png" alt="Nexa-net Logo" width="100" />
  <h1>Nexa-net</h1>
  <p><b><i>Decentralized M2M Communication Infrastructure for AI Agent Networks</i></b></p>
  <p>
    <img src="https://img.shields.io/badge/License-MIT-blue.svg?style=for-the-badge" alt="License"/>
    <img src="https://img.shields.io/badge/Version-v0.1.0--alpha-brightgreen.svg?style=for-the-badge" alt="Version"/>
    <img src="https://img.shields.io/badge/Rust-1.70%2B-orange.svg?style=for-the-badge" alt="Rust"/>
    <img src="https://img.shields.io/badge/Status-Experimental-orange.svg?style=for-the-badge" alt="Status"/>
  </p>
  
  **中文版** | **[English](#overview)**
  
  📚 **文档**: [架构设计](docs/ARCHITECTURE.md) | [API 参考](docs/API_REFERENCE.md) | [开发指南](docs/DEVELOPER_GUIDE.md)
</div>

---

## ⚡ What is Nexa-net?

**Nexa-net** 是一个去中心化的 M2M (Machine-to-Machine) 通信基础设施，为 AI Agent 网络提供安全、高效、经济的服务发现与调用能力。它采用 **Sidecar Proxy** 模式实现非侵入式集成，让现有 AI 应用无需修改代码即可接入去中心化网络。

---

## 🔥 Key Features

### 🔐 Layer 1: Identity & Trust
去中心化身份与零信任安全基础：
- **Nexa-DID** - 基于 W3C DID 规范的去中心化身份体系
- **mTLS** - 双向 TLS 认证，确保通信安全
- **Verifiable Credentials** - 可验证凭证，支持跨域信任传递

### 🔍 Layer 2: Semantic Discovery
语义驱动的智能服务发现：
- **Capability Schema** - 统一的能力描述规范
- **Semantic Router** - 基于向量相似度的多因子路由
- **DHT** - 分布式哈希表，支持大规模节点发现
- **ONNX Embedding** - 本地向量化，支持语义匹配

### ⚡ Layer 3: Transport & Protocol
高性能传输与协议协商：
- **Frame Protocol** - 12 字节精简帧头，零拷贝设计
- **Streaming RPC** - 双向流式 RPC，支持大规模并发
- **LZ4/Zstd** - 高性能压缩，降低带宽消耗
- **SYN-NEXA/ACK-SCHEMA** - 智能协议协商握手

### 💰 Layer 4: Economy
微交易经济与资源管理：
- **State Channel** - Layer 2 状态通道，支持高频微交易
- **Micro-Receipt** - 轻量级交易凭证，链下结算
- **Budget Controller** - 预算控制，防止资源滥用

---

## 📐 Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        Nexa-net Four-Layer Architecture                  │
├─────────────────────────────────────────────────────────────────────────┤
│  Layer 1: Identity    │ Nexa-DID, mTLS, Verifiable Credentials          │
│  Layer 2: Discovery   │ Capability Schema, Semantic Router, DHT          │
│  Layer 3: Transport   │ Frame Protocol, Streaming RPC, LZ4/Zstd          │
│  Layer 4: Economy     │ State Channel, Micro-Receipt, Token              │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 🚀 Quick Start

### Installation

```bash
# Clone the repository
git clone https://github.com/ouyangyipeng/Nexa-net.git
cd Nexa-net

# Build
cargo build --release

# Run tests
cargo test --lib
```

### Run Nexa-Proxy

```bash
# Start the proxy service
cargo run --bin nexa-proxy --release
```

The proxy service will start on:
- REST API: `http://127.0.0.1:7070/api/v1`
- gRPC: `127.0.0.1:7071`

---

## 💻 SDK Usage

```rust
use nexa_net::{
    api::sdk::{NexaClient, NexaClientBuilder},
    types::Did,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Build client
    let client = NexaClientBuilder::new()
        .endpoint("http://localhost:7070")
        .timeout_ms(5000)
        .budget(100000)
        .build();

    // Discover services by intent
    let routes = client.discover("translate text to French", 5).await?;
    
    // Make a network call
    let response = client.call("translate", b"Hello, world!".to_vec()).await?;
    
    Ok(())
}
```

---

## 📦 Project Structure

```
src/
├── identity/           # Layer 1: Identity & Zero-Trust
│   ├── did.rs          # Nexa-DID implementation
│   ├── key_management.rs # Ed25519/X25519 key management
│   └── credential.rs   # Verifiable Credentials
├── discovery/          # Layer 2: Semantic Discovery
│   ├── capability.rs   # Capability registry
│   ├── router.rs       # Multi-factor semantic router
│   ├── vectorizer.rs   # Semantic vectorization
│   └── embedding/      # Embedding module
│       ├── mod.rs      # Embedder trait
│       ├── mock.rs     # Mock Embedder
│       └── onnx.rs     # ONNX Runtime Embedder
├── transport/          # Layer 3: Transport Protocol
│   ├── frame.rs        # 12-byte frame protocol
│   ├── stream.rs       # Multiplexed streams
│   ├── rpc.rs          # Streaming RPC engine
│   └── negotiator.rs   # SYN-NEXA/ACK-SCHEMA handshake
├── economy/            # Layer 4: Economy Layer
│   ├── channel.rs      # State channel management
│   ├── receipt.rs      # Micro-transaction receipts
│   └── budget.rs       # Budget controller
├── storage/            # Persistence Layer
│   ├── mod.rs          # Storage traits
│   ├── memory.rs       # In-memory store
│   ├── postgres.rs     # PostgreSQL backend
│   └── redis.rs        # Redis cache
├── security/           # Security Module
│   ├── audit.rs        # Audit logging
│   ├── key_rotation.rs # Key rotation
│   ├── rate_limit.rs   # Rate limiting
│   └── secure_storage.rs # Encrypted storage
├── proxy/              # Nexa-Proxy daemon
│   ├── server.rs       # REST/gRPC server
│   └── config.rs       # Configuration
└── api/                # SDK Interface
    └── sdk.rs          # NexaClient SDK
```

---

## 📊 Frame Protocol

```
┌───────────────────────────────────────────────────────────────┐
│                    Nexa-net Frame Format                       │
├───────────────────────────────────────────────────────────────┤
│  Magic (4B) │ Type (1B) │ Flags (1B) │ StreamID (2B) │ Len (4B) │
├───────────────────────────────────────────────────────────────┤
│  0x4E584E54 │  DATA     │  0x00     │    0x0001     │  1024   │
│  "NXNT"     │  ACK      │  COMPRESS │               │         │
│             │  RST      │  PRIORITY │               │         │
└───────────────────────────────────────────────────────────────┘
```

---

## 🧪 Testing

```bash
# Run unit tests
cargo test --lib

# Run integration tests
cargo test --test '*'

# Run with all features
cargo test --all-features

# Run benchmarks
cargo bench
```

---

## 📚 Documentation

| Document | Description |
|----------|-------------|
| [Architecture](docs/ARCHITECTURE.md) | System architecture and design decisions |
| [API Reference](docs/API_REFERENCE.md) | REST/gRPC/SDK API documentation |
| [Developer Guide](docs/DEVELOPER_GUIDE.md) | Development setup and contribution guide |
| [Security](docs/SECURITY.md) | Security model and best practices |
| [Deployment](docs/DEPLOYMENT.md) | Production deployment guide |

---

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guide](docs/DEVELOPER_GUIDE.md#contributing) for details.

---

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---

## 🙏 Acknowledgments

- Inspired by [libp2p](https://libp2p.io/) for P2P networking patterns
- [gRPC](https://grpc.io/) for RPC design patterns
- [W3C DID](https://www.w3.org/TR/did-core/) for identity specification