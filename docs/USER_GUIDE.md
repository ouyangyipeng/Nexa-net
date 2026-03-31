# Nexa-net 用户指南

本指南帮助您快速安装和使用 Nexa-net。

## 目录

- [系统要求](#系统要求)
- [安装方式](#安装方式)
- [快速开始](#快速开始)
- [功能配置](#功能配置)
- [示例代码](#示例代码)
- [常见问题](#常见问题)

---

## 系统要求

- **操作系统**: Linux (Ubuntu 22.04+), macOS, Windows (WSL2)
- **Rust**: 1.75 或更高版本
- **硬件**: 
  - CPU: 4核心以上
  - RAM: 8GB 以上
  - GPU: (可选) NVIDIA GPU 用于 ONNX 推理加速

---

## 安装方式

### 1. 从源码构建

```bash
# 克隆仓库
git clone https://github.com/nexa-net/nexa-net.git
cd nexa-net

# 构建项目
cargo build --release

# 运行测试
cargo test
```

### 2. 启用可选功能

```bash
# 启用 ONNX Embedding 支持
cargo build --release --features embedding-onnx

# 启用 PostgreSQL 存储
cargo build --release --features storage-postgres

# 启用 Redis 缓存
cargo build --release --features storage-redis

# 启用所有存储功能
cargo build --release --features storage-full
```

### 3. 安装 Nexa-Proxy

```bash
# 构建 proxy 二进制
cargo build --release --bin nexa-proxy

# 运行 proxy
./target/release/nexa-proxy
```

---

## 快速开始

### 1. 创建身份

```rust
use nexa_net::identity::{KeyPair, Did};

// 生成密钥对
let keypair = KeyPair::generate()?;

// 创建 DID
let did = Did::from_public_key(&keypair.public_key());
println!("My DID: {}", did);
```

### 2. 注册能力

```rust
use nexa_net::discovery::{CapabilityRegistry, Vectorizer};
use nexa_net::types::{CapabilitySchema, ServiceMetadata, Did};

let mut registry = CapabilityRegistry::new();

let schema = CapabilitySchema {
    version: "1.0".to_string(),
    metadata: ServiceMetadata {
        did: Did::new("did:nexa:my-service"),
        name: "translation-service".to_string(),
        description: "English to Chinese translation".to_string(),
        tags: vec!["translation".to_string(), "nlp".to_string()],
    },
    endpoints: vec![],
};

registry.register(schema)?;
```

### 3. 语义发现

```rust
use nexa_net::discovery::{SemanticRouter, Vectorizer, VectorizerBuilder};

// 创建向量化器
let vectorizer = Vectorizer::new();

// 向量化意图
let vector = vectorizer.vectorize("translate English text to Chinese")?;

// 创建语义路由器
let router = SemanticRouter::new(registry);

// 发现服务
let routes = router.discover("translate document", RouteContext::default()).await?;
```

### 4. 使用 SDK 客户端

```rust
use nexa_net::api::sdk::NexaClientBuilder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = NexaClientBuilder::new()
        .endpoint("http://127.0.0.1:7070")
        .timeout_ms(30000)
        .budget(100)
        .build();

    // 健康检查
    let healthy = client.health_check().await?;
    println!("Server healthy: {}", healthy);

    // 发现服务
    let services = client.discover("translation", 10).await?;
    for service in services {
        println!("Found: {:?}", service);
    }

    Ok(())
}
```

---

## 功能配置

### Embedding 配置

#### 使用 Mock Embedder (默认，用于测试)

```rust
use nexa_net::discovery::{Vectorizer, VectorizerBuilder};

// 默认使用 Mock Embedder
let vectorizer = Vectorizer::new();

// 或显式配置
let vectorizer = VectorizerBuilder::new()
    .mock(384)  // 384 维向量
    .build()?;
```

#### 使用 ONNX Embedder (生产环境)

```bash
# 1. 下载模型
./scripts/download_embedding_model.sh all-MiniLM-L6-v2

# 2. 启用 feature 编译
cargo build --release --features embedding-onnx
```

```rust
use nexa_net::discovery::VectorizerBuilder;
use std::path::PathBuf;

let vectorizer = VectorizerBuilder::new()
    .onnx(
        PathBuf::from("models/all-MiniLM-L6-v2/model.onnx"),
        512  // 最大序列长度
    )
    .build()?;
```

### 存储配置

#### 内存存储 (默认)

```rust
use nexa_net::storage::MemoryStore;

let store = MemoryStore::default_store();
```

#### PostgreSQL 存储

```bash
# 启用 feature
cargo build --release --features storage-postgres
```

```rust
// 配置连接
let config = StorageConfig {
    postgres_url: Some("postgresql://user:pass@localhost/nexa".to_string()),
    ..Default::default()
};
```

### 安全配置

```rust
use nexa_net::security::{
    SecurityConfig, AuditLogger, RateLimiter, 
    RateLimitConfig, KeyRotator, KeyRotationPolicy
};

// 审计日志
let audit = AuditLogger::with_logging();

// 速率限制
let rate_limiter = RateLimiter::new(RateLimitConfig {
    requests_per_minute: 60,
    requests_per_hour: 1000,
    ..Default::default()
});

// 密钥轮换
let rotator = KeyRotator::new(KeyRotationPolicy {
    rotation_interval_days: 90,
    auto_rotate: true,
    ..Default::default()
});
```

---

## 示例代码

### 完整示例：创建 Agent 并注册服务

```rust
use nexa_net::{
    identity::KeyPair,
    discovery::{CapabilityRegistry, SemanticRouter, VectorizerBuilder},
    types::{CapabilitySchema, ServiceMetadata, Did, EndpointDefinition},
    economy::ChannelManager,
    storage::MemoryStore,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 创建身份
    let keypair = KeyPair::generate()?;
    let did = Did::from_public_key(&keypair.public_key());
    println!("Agent DID: {}", did);

    // 2. 初始化存储
    let store = MemoryStore::default_store();

    // 3. 创建能力注册表
    let mut registry = CapabilityRegistry::new();

    // 4. 注册服务能力
    let schema = CapabilitySchema {
        version: "1.0".to_string(),
        metadata: ServiceMetadata {
            did: did.clone(),
            name: "text-processor".to_string(),
            description: "Text processing and analysis service".to_string(),
            tags: vec!["nlp".to_string(), "text".to_string()],
        },
        endpoints: vec![],
    };
    registry.register(schema)?;

    // 5. 创建语义路由器
    let vectorizer = VectorizerBuilder::new()
        .mock(384)
        .build()?;
    
    let router = SemanticRouter::new(registry);

    // 6. 发现服务
    let routes = router.discover("process text document", Default::default()).await?;
    println!("Found {} matching services", routes.len());

    // 7. 管理支付通道
    let channel_manager = ChannelManager::default();
    let channel = channel_manager.open(
        did.clone(),
        Did::new("did:nexa:other-agent"),
        1000,  // deposit A
        500,   // deposit B
    )?;
    println!("Opened channel: {}", channel.id);

    Ok(())
}
```

---

## 运行基准测试

```bash
# 运行所有基准测试
cargo bench

# 运行特定基准测试
cargo bench -- identity_benches
cargo bench -- discovery_benches
cargo bench -- economy_benches
cargo bench -- storage_benches
```

---

## 常见问题

### Q: 如何选择 Embedding 后端？

| 场景 | 推荐后端 |
|------|----------|
| 开发/测试 | Mock Embedder |
| 生产环境 (低延迟) | ONNX Runtime (本地) |
| 生产环境 (高精度) | API Embedder (远程) |

### Q: 如何更新密钥？

```rust
use nexa_net::security::{KeyRotator, KeyRotationPolicy};

let rotator = KeyRotator::new(KeyRotationPolicy {
    rotation_interval_days: 30,  // 每 30 天轮换
    auto_rotate: true,
    ..Default::default()
});

// 检查是否需要轮换
if rotator.needs_rotation("key-1").await? {
    rotator.mark_rotated("key-1").await?;
}
```

### Q: 如何处理速率限制？

```rust
use nexa_net::security::{RateLimiter, RateLimitKey, RateLimitResult};

let limiter = RateLimiter::default_limiter();
let key = RateLimitKey::Did("did:nexa:user".to_string());

match limiter.check(&key).await? {
    RateLimitResult::Allowed => {
        // 处理请求
    }
    RateLimitResult::Denied { reason, retry_after } => {
        println!("Rate limited: {}. Retry in {}s", reason, retry_after);
    }
}
```

### Q: 如何查看审计日志？

```rust
use nexa_net::security::{AuditLogger, MemoryAuditSink};
use std::sync::Arc;

let sink = Arc::new(MemoryAuditSink::new(1000));
let logger = AuditLogger::new(sink.clone());

// 记录事件
logger.log_auth_success("did:nexa:user", AuthMethod::Signature, None)?;

// 查看事件
let events = sink.get_events().await;
for event in events {
    println!("{:?}", event);
}
```

---

## 更多资源

- [API 参考](./API_REFERENCE.md)
- [架构设计](./ARCHITECTURE.md)
- [协议规范](./PROTOCOL_SPEC.md)
- [部署指南](./DEPLOYMENT.md)
- [安全设计](./SECURITY.md)

---

## 获取帮助

- GitHub Issues: https://github.com/nexa-net/nexa-net/issues
- 文档: https://docs.nexa-net.io
- 社区: https://discord.gg/nexa-net