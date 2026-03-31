# Nexa-net 后续增强计划

## 概述

本文档规划 Nexa-net 的四个关键增强功能，旨在将项目从原型阶段推进到生产就绪状态。

## 增强功能概览

| 功能 | 优先级 | 复杂度 | 预计工作量 |
|------|--------|--------|------------|
| 真实 Embedding 模型集成 | P0 | 中 | 3-5 天 |
| 持久化存储 (Redis/PostgreSQL) | P0 | 高 | 5-7 天 |
| 性能基准测试 | P1 | 低 | 2-3 天 |
| 安全审计与密钥管理优化 | P0 | 高 | 4-6 天 |

---

## 1. 真实 Embedding 模型集成

### 1.1 当前问题

现有的 [`Vectorizer`](src/discovery/vectorizer.rs:46) 使用简单的 hash-based 方法：

```rust
// 当前实现 - 仅用于测试
pub fn vectorize(&self, text: &str) -> Result<SemanticVector> {
    // Simple hash-based vectorization for testing
    // TODO: Replace with actual embedding model
    let mut data = vec![0.0f32; self.dimensions];
    for (i, byte) in text.as_bytes().iter().enumerate() {
        let idx = (i + *byte as usize) % self.dimensions;
        data[idx] += (*byte as f32 / 255.0) - 0.5;
    }
    // ...
}
```

**问题**：
- 无法捕捉语义相似性
- 相似意图可能产生完全不同的向量
- 依赖测试配置 (`min_similarity: 0.0`) 才能工作

### 1.2 解决方案

#### 方案 A: ONNX Runtime 本地推理 (推荐)

```
优点：
- 无外部依赖，部署简单
- 低延迟，适合边缘场景
- 支持 CPU/GPU 推理

缺点：
- 模型文件较大 (~100MB-500MB)
- 首次加载较慢
```

**技术选型**：
- 模型: `all-MiniLM-L6-v2` (384 维，~80MB)
- 运行时: `ort` (ONNX Runtime Rust bindings)
- 量化: 可选 INT8 量化减小模型体积

#### 方案 B: 远程 Embedding API

```
优点：
- 无本地计算资源需求
- 可使用更强大的模型
- 模型更新无需重新部署

缺点：
- 网络延迟
- 依赖外部服务
- 成本考虑
```

**技术选型**：
- OpenAI Embeddings API (`text-embedding-3-small`)
- 自托管 FastEmbed 服务

### 1.3 实现计划

```mermaid
graph TD
    A[定义 Embedder Trait] --> B[实现 ONNX Embedder]
    A --> C[实现 API Embedder]
    B --> D[模型下载/打包脚本]
    C --> E[HTTP 客户端集成]
    D --> F[Vectorizer 重构]
    E --> F
    F --> G[配置系统更新]
    G --> H[集成测试]
    H --> I[性能基准测试]
```

### 1.4 代码结构

```
src/discovery/
├── vectorizer.rs          # 现有文件，重构为 trait
├── embedding/
│   ├── mod.rs             # Embedder trait 定义
│   ├── onnx.rs            # ONNX Runtime 实现
│   ├── api.rs             # 远程 API 实现
│   └── cache.rs           # 向量缓存层
└── config.rs              # Embedding 配置
```

### 1.5 API 设计

```rust
/// Embedder trait - 支持多种后端
pub trait Embedder: Send + Sync {
    /// 将文本转换为向量
    fn embed(&self, text: &str) -> Result<Vec<f32>>;
    
    /// 批量嵌入
    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>>;
    
    /// 获取向量维度
    fn dimensions(&self) -> usize;
    
    /// 获取模型名称
    fn model_name(&self) -> &str;
}

/// 配置
pub struct EmbeddingConfig {
    pub backend: EmbeddingBackend,
    pub model: String,
    pub cache_size: usize,
    pub batch_size: usize,
}

pub enum EmbeddingBackend {
    Onnx { model_path: PathBuf },
    Api { endpoint: String, api_key: Option<String> },
    Mock, // 用于测试
}
```

---

## 2. 持久化存储

### 2.1 当前问题

- `CapabilityRegistry`: 内存 HashMap，重启丢失
- `ChannelManager`: 内存存储，无持久化
- `SemanticDHT`: 内存实现，无法扩展
- 无事务支持，无数据恢复能力

### 2.2 存储需求分析

| 数据类型 | 访问模式 | 持久化要求 | 推荐存储 |
|----------|----------|------------|----------|
| Capability Registry | 读写频繁，需查询 | 持久化 | Redis + PostgreSQL |
| Channel State | 事务性，需一致性 | 强持久化 | PostgreSQL |
| Semantic Vectors | 高维向量，相似搜索 | 可重建 | Redis + 向量索引 |
| Node Status | 时序数据，监控 | 时序存储 | Redis (TTL) |
| Session/Cache | 临时数据 | 可丢失 | Redis |

### 2.3 架构设计

```mermaid
graph TB
    subgraph Application
        A[Nexa-Proxy]
    end
    
    subgraph Cache Layer
        R[Redis]
    end
    
    subgraph Persistence Layer
        P[(PostgreSQL)]
    end
    
    A -->|Hot Data| R
    A -->|Durable Data| P
    R -->|Async Sync| P
```

### 2.4 数据模型

#### PostgreSQL Schema

```sql
-- 能力注册表
CREATE TABLE capabilities (
    did VARCHAR(128) PRIMARY KEY,
    schema JSONB NOT NULL,
    quality JSONB DEFAULT '{}',
    available BOOLEAN DEFAULT true,
    registered_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW()
);

CREATE INDEX idx_capabilities_tags ON capabilities USING GIN ((schema->'metadata'->'tags'));
CREATE INDEX idx_capabilities_available ON capabilities(available);

-- 状态通道
CREATE TABLE channels (
    id VARCHAR(64) PRIMARY KEY,
    party_a VARCHAR(128) NOT NULL,
    party_b VARCHAR(128) NOT NULL,
    balance_a BIGINT NOT NULL,
    balance_b BIGINT NOT NULL,
    state VARCHAR(32) NOT NULL,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW(),
    settlement_deadline TIMESTAMP,
    dispute JSONB
);

CREATE INDEX idx_channels_party ON channels(party_a, party_b);
CREATE INDEX idx_channels_state ON channels(state);

-- 微收据
CREATE TABLE receipts (
    id SERIAL PRIMARY KEY,
    call_id VARCHAR(64) NOT NULL,
    payer_did VARCHAR(128) NOT NULL,
    payee_did VARCHAR(128) NOT NULL,
    amount BIGINT NOT NULL,
    signature BYTEA,
    created_at TIMESTAMP DEFAULT NOW()
);

CREATE INDEX idx_receipts_call ON receipts(call_id);
CREATE INDEX idx_receipts_payer ON receipts(payer_did);
```

#### Redis Schema

```
# 能力缓存
cap:{did} -> JSON (TTL: 5min)

# 向量缓存
vec:{text_hash} -> binary_vector (TTL: 1hour)

# 节点状态
node:{did}:status -> JSON (TTL: 30s)
node:{did}:load -> float (TTL: 30s)

# 会话
session:{session_id} -> JSON (TTL: 24h)

# 分布式锁
lock:channel:{channel_id} -> token (TTL: 10s)
```

### 2.5 代码结构

```
src/storage/
├── mod.rs                 # Storage trait 定义
├── postgres/
│   ├── mod.rs
│   ├── capabilities.rs    # 能力存储
│   ├── channels.rs        # 通道存储
│   └── receipts.rs        # 收据存储
├── redis/
│   ├── mod.rs
│   ├── cache.rs           # 缓存层
│   ├── vectors.rs         # 向量缓存
│   └── locks.rs           # 分布式锁
└── config.rs              # 存储配置
```

### 2.6 API 设计

```rust
/// 存储后端 trait
#[async_trait]
pub trait CapabilityStore: Send + Sync {
    async fn register(&self, schema: CapabilitySchema) -> Result<()>;
    async fn unregister(&self, did: &str) -> Result<()>;
    async fn get(&self, did: &str) -> Result<Option<RegisteredCapability>>;
    async fn find_by_tags(&self, tags: &[String]) -> Result<Vec<CapabilitySchema>>;
    async fn list_all(&self) -> Result<Vec<RegisteredCapability>>;
}

#[async_trait]
pub trait ChannelStore: Send + Sync {
    async fn save(&self, channel: &Channel) -> Result<()>;
    async fn load(&self, id: &str) -> Result<Option<Channel>>;
    async fn list_open(&self) -> Result<Vec<Channel>>;
    async fn list_for_peer(&self, did: &Did) -> Result<Vec<Channel>>;
}
```

---

## 3. 性能基准测试

### 3.1 测试场景

| 场景 | 指标 | 目标 |
|------|------|------|
| 服务发现 | P99 延迟 | < 50ms |
| 通道开启 | 吞吐量 | > 1000 TPS |
| 向量相似度计算 | QPS | > 10000 |
| 能力注册 | P99 延迟 | < 10ms |
| 端到端调用 | P99 延迟 | < 100ms |

### 3.2 基准测试框架

使用 `criterion` 进行微基准测试，`tokio-test` 进行异步测试。

### 3.3 代码结构

```
benches/
├── discovery_bench.rs     # 发现层基准
├── channel_bench.rs       # 通道操作基准
├── vector_bench.rs        # 向量计算基准
├── serialization_bench.rs # 序列化基准
└── e2e_bench.rs           # 端到端基准
```

### 3.4 CI 集成

```yaml
# .github/workflows/benchmark.yml
name: Benchmarks
on: [push, pull_request]
jobs:
  bench:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Run benchmarks
        run: cargo bench -- --save-baseline main
      - name: Compare with baseline
        run: cargo bench -- --baseline main
```

---

## 4. 安全审计与密钥管理优化

### 4.1 当前问题

1. **密钥存储**: 密钥以明文形式存储在内存中
2. **密钥轮换**: 无自动轮换机制
3. **密钥派生**: 无分层密钥派生
4. **审计日志**: 无安全事件记录

### 4.2 安全增强方案

#### 4.2.1 密钥存储安全

```rust
/// 安全密钥存储
pub struct SecureKeyStorage {
    /// 加密密钥 (从环境变量或 KMS 获取)
    master_key: [u8; 32],
    /// 存储路径
    storage_path: PathBuf,
}

impl SecureKeyStorage {
    /// 存储密钥 (加密后)
    pub fn store(&self, key_id: &str, key: &[u8]) -> Result<()>;
    
    /// 加载密钥 (解密后，内存中安全处理)
    pub fn load(&self, key_id: &str) -> Result<ZeroizedBytes>;
    
    /// 轮换密钥
    pub fn rotate(&self, key_id: &str) -> Result<()>;
}
```

#### 4.2.2 密钥轮换策略

```rust
pub struct KeyRotationPolicy {
    /// 轮换周期
    rotation_period: Duration,
    /// 旧密钥保留时间 (用于验证旧签名)
    grace_period: Duration,
    /// 自动轮换
    auto_rotate: bool,
}

impl KeyRotationPolicy {
    /// 检查是否需要轮换
    pub fn should_rotate(&self, key: &KeyMetadata) -> bool;
    
    /// 执行轮换
    pub fn rotate(&self, storage: &SecureKeyStorage) -> Result<KeyPair>;
}
```

#### 4.2.3 审计日志

```rust
/// 安全审计事件
pub enum AuditEvent {
    KeyGenerated { key_id: String, timestamp: DateTime<Utc> },
    KeyRotated { key_id: String, old_version: u32, new_version: u32 },
    KeyAccessed { key_id: String, accessor: String },
    AuthenticationSuccess { did: String, method: AuthMethod },
    AuthenticationFailure { did: String, reason: String },
    ChannelOpened { channel_id: String, parties: (String, String) },
    ChannelClosed { channel_id: String, final_state: ChannelState },
}

/// 审计日志器
pub struct AuditLogger {
    sink: Box<dyn AuditSink>,
}

pub trait AuditSink: Send + Sync {
    fn log(&self, event: AuditEvent) -> Result<()>;
}
```

### 4.3 安全检查清单

- [ ] 密钥不在日志中输出
- [ ] 敏感数据使用 `zeroize` 清理
- [ ] TLS 1.3 强制使用
- [ ] 证书验证严格
- [ ] 输入验证完整
- [ ] 速率限制实现
- [ ] 审计日志完整
- [ ] 密钥轮换自动化

### 4.4 代码结构

```
src/security/
├── mod.rs
├── key_storage.rs         # 安全密钥存储
├── key_rotation.rs        # 密钥轮换
├── audit.rs               # 审计日志
├── tls.rs                 # TLS 配置
└── rate_limit.rs          # 速率限制
```

---

## 5. 实施路线图

### Phase 1: 基础设施 (Week 1-2)

1. **存储层实现**
   - [ ] 定义 Storage traits
   - [ ] 实现 PostgreSQL 后端
   - [ ] 实现 Redis 缓存层
   - [ ] 数据库迁移脚本

2. **安全基础**
   - [ ] 安全密钥存储实现
   - [ ] 审计日志框架
   - [ ] TLS 配置优化

### Phase 2: Embedding 集成 (Week 3)

1. **Embedder 实现**
   - [ ] Embedder trait 定义
   - [ ] ONNX Runtime 集成
   - [ ] 模型下载脚本
   - [ ] 向量缓存层

2. **集成测试**
   - [ ] 端到端测试
   - [ ] 性能验证

### Phase 3: 性能优化 (Week 4)

1. **基准测试**
   - [ ] 微基准测试
   - [ ] 端到端基准
   - [ ] CI 集成

2. **优化**
   - [ ] 热点分析
   - [ ] 性能调优
   - [ ] 文档更新

---

## 6. 依赖更新

### Cargo.toml 新增依赖

```toml
[dependencies]
# Embedding
ort = "2.0"
ndarray = "0.15"

# Storage
sqlx = { version = "0.7", features = ["runtime-tokio", "postgres", "json"] }
redis = { version = "0.25", features = ["tokio-comp", "connection-manager"] }

# Security
zeroize = "1.7"
secrecy = "0.8"

# Benchmarking (dev)
[dev-dependencies]
criterion = { version = "0.5", features = ["async_tokio"] }
```

---

## 7. 配置示例

### config.yaml

```yaml
# Nexa-Proxy 配置
server:
  http:
    bind: "0.0.0.0:7070"
  grpc:
    bind: "0.0.0.0:7071"

# Embedding 配置
embedding:
  backend: "onnx"
  model: "all-MiniLM-L6-v2"
  model_path: "/var/lib/nexa/models/"
  cache_size: 10000
  batch_size: 32

# 存储配置
storage:
  postgres:
    url: "postgres://nexa:nexa@localhost:5432/nexa"
    max_connections: 10
  redis:
    url: "redis://localhost:6379"
    pool_size: 10

# 安全配置
security:
  key_storage:
    path: "/var/lib/nexa/keys"
    encryption: "aes-256-gcm"
  key_rotation:
    enabled: true
    period: "30d"
    grace_period: "7d"
  audit:
    enabled: true
    path: "/var/log/nexa/audit.log"
```

---

## 8. 验收标准

### Embedding 集成

- [ ] 语义相似度测试通过 (相似文本 > 0.7)
- [ ] 批量嵌入性能 > 1000 texts/sec
- [ ] 内存使用 < 500MB

### 持久化存储

- [ ] 数据持久化测试通过
- [ ] 故障恢复测试通过
- [ ] 事务一致性测试通过

### 性能基准

- [ ] 所有基准测试通过
- [ ] 无性能退化 (> 10%)
- [ ] CI 集成完成

### 安全审计

- [ ] 密钥存储加密验证
- [ ] 密钥轮换测试通过
- [ ] 审计日志完整
- [ ] 安全扫描无高危问题

---

*文档版本: 1.0*
*创建日期: 2026-03-31*
*作者: Nexa-net Team*