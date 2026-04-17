# Nexa-net ROADMAP — 12 Phase 工业级重构历史

> **创建时间:** 2026-04-16
> **状态:** Phase 1-12 全部完成 ✅

---

## 📊 重构总览

| Phase | 名称 | 关键变更 | 状态 |
|-------|------|---------|------|
| Phase 1 | 类型系统修复 | Did 类型冲突解决，统一为 `Did(String)` | ✅ |
| Phase 2 | Identity Layer 修复 | DID document 序列化，credential 签名验证 | ✅ |
| Phase 3 | Discovery Layer 修复 | DashMap 替换 RwLock+HashMap，DHT 连接管理 | ✅ |
| Phase 4 | Transport Layer 修复 | Frame protocol，stream management，RPC engine | ✅ |
| Phase 5 | Security Layer 实现 | XOR→AES-256-GCM，SecurityManager 协调器 | ✅ |
| Phase 6 | API Layer 实现 | 7 个 REST API 端点 + gRPC health service | ✅ |
| Phase 7 | Security 增强 | 审计日志、密钥轮换、速率限制 middleware | ✅ |
| Phase 8 | SDK 实现 | NexaClient/NexaClientBuilder Rust SDK | ✅ |
| Phase 9 | Economy Layer 修复 | State Channel，Micro-Receipt，Budget Controller | ✅ |
| Phase 10 | Storage 实现 | Memory/RocksDB/PostgreSQL/Redis 后端 | ✅ |
| Phase 11 | Protocol Layer 修复 | Protobuf 消息序列化，跨层协议绑定 | ✅ |
| Phase 12 | 文档同步与打磨 | Clippy 0 warnings，433 tests passed，文档同步 | ✅ |

---

## 🔑 关键决策记录

### Decision 1: XOR → AES-256-GCM (Phase 5)

**背景:** 原始实现使用 XOR 加密存储密钥数据，XOR 加密的安全性极低，不符合工业级要求。

**决策:** 替换为 AES-256-GCM 加密，使用 `aes-gcm` crate 实现。

**Trade-offs:**
- ✅ 安全性：AES-256-GCM 提供 256-bit 加密强度 + 认证标签
- ✅ 认证：GCM 模式内置完整性验证
- ⚠️ 性能：AES 比 XOR 略慢，但差异在微秒级，对密钥存储场景无影响
- ⚠️ 依赖：新增 `aes-gcm` crate 依赖

**代码位置:** `src/security/secure_storage.rs`

### Decision 2: 两种 Did 类型冲突 (Phase 1)

**背景:** `src/types.rs` 中存在两种 Did 定义：`Did(String)` (newtype) 和 `Did { method, identifier }` (struct)，导致跨模块兼容性问题。

**决策:** 统一为 `Did(String)` newtype，保留辅助方法 `method()` 和 `identifier()`。

**Trade-offs:**
- ✅ 简洁：单一类型，减少认知负担
- ✅ 兼容：所有模块使用同一类型
- ⚠️ 解析：需要从字符串中解析 method 和 identifier

**代码位置:** `src/types.rs`

### Decision 3: DashMap 替换 RwLock+HashMap (Phase 3)

**背景:** `SemanticDHT` 中使用 `RwLock<HashMap<...>>` 存储节点和向量数据，高并发下锁竞争严重。

**决策:** 替换为 `DashMap<...>`，利用分片锁减少竞争。

**Trade-offs:**
- ✅ 并发：DashMap 分片锁，读写并发度大幅提升
- ✅ API：DashMap API 与 HashMap 类似，迁移成本低
- ⚠️ 内存：DashMap 分片带来少量额外内存开销

**代码位置:** `src/discovery/semantic_dht.rs`

### Decision 4: Error variant boxing (Phase 12)

**背景:** `Error::Grpc(#[from] tonic::Status)` variant 占 176 bytes，导致所有 `Result<T>` 的 Err variant 过大（136 个 clippy `result_large_err` 警告）。

**决策:** 将 `Grpc` variant 改为 `Grpc(Box<tonic::Status>)`，手动实现 `From<tonic::Status>`。

**Trade-offs:**
- ✅ 内存：Error enum 从 176+ bytes 缩减到合理大小
- ✅ 性能：减少 Result 传递时的内存拷贝
- ⚠️ 使用：匹配 `Error::Grpc(e)` 时需要 `*e` 解引用

**代码位置:** `src/error.rs`

---

## 📝 从错误中学习

### Lesson 1: 不要使用 XOR 加密

XOR 加密在密钥存储场景中完全不可接受。任何时候涉及到密钥/凭证存储，必须使用工业级加密算法（AES-256-GCM 或 ChaCha20-Poly1305）。

### Lesson 2: 类型冲突要及早解决

两种 `Did` 类型共存导致跨模块 API 不一致。在项目初期就应确定唯一的类型定义，避免后期重构成本。

### Lesson 3: Clippy 修复应手动进行

使用 Python 脚本/sed 批量修复 clippy 警告时，容易产生语法错误（如 `pub async fn call_full(    pub async fn call_full(...)` 双重声明）。应该优先使用 `cargo clippy --fix`，然后手动处理剩余问题。

### Lesson 4: Error enum 设计要考虑大小

当包含大型 variant（如 `tonic::Status`）时，应该使用 `Box<T>` 包装，避免整个 Result 类型过大导致性能问题。

### Lesson 5: #[derive(Default)] 不适用于有 cfg feature 字段的 struct

`StorageConfig` 有 `#[cfg(feature = "storage-rocksdb")]` 等条件字段，`#[derive(Default)]` 生成的 impl 在不同 feature 组合下行为不一致。应手动实现 Default 或使用 `#[allow(clippy::derivable_impls)]`。

### Lesson 6: ProxyState 中 registry 和 router 必须共享同一个 Arc<RwLock> 实例

**背景:** `ProxyState::new()` 原始实现中，`registry` 和 `SemanticRouter` 分别创建了各自独立的 `CapabilityRegistry` 实例。这导致 `/v1/register` 写入的数据存入了 router 内部的 registry，而 `/v1/discover` 读取的是 ProxyState.registry 中的另一个实例，二者数据互不可见。

**决策:** 修改 `ProxyState::new()` 使 registry 和 router 共享同一个 `Arc<RwLock<CapabilityRegistry>>`：
```rust
let registry = Arc::new(RwLock::new(CapabilityRegistry::new()));
let router = Arc::new(RwLock::new(SemanticRouter::with_shared(
    registry.clone(),  // 共享同一个 Arc
    node_status.clone(),
)));
```

**Trade-offs:**
- ✅ 数据一致性：register 写入立即可见于 discover
- ✅ 内存效率：避免两个 registry 的数据冗余
- ⚠️ API 变更：新增 `SemanticRouter::with_shared()` 方法替代原来的 `SemanticRouter::new(registry)`（后者会 consume registry）

**代码位置:** `src/proxy/server.rs`, `src/discovery/router.rs`

---

## 🔄 变更历史

| 日期 | 变更 | 作者 |
|------|------|------|
| 2026-04-16 | Phase 11-12 完成：45 benchmarks (含 REST API 延迟)，36 TODO→NOTE 清理，文档同步，485 tests，Clippy 0 warnings | Owen + AI |
| 2026-04-15 | Phase 11 完成：Protocol Layer 修复 | Owen + AI |
| 2026-04-14 | Phase 10 完成：Storage 实现 | Owen + AI |
| 2026-04-13 | Phase 9 完成：Economy Layer 修复 | Owen + AI |
| 2026-04-12 | Phase 8 完成：SDK 实现 | Owen + AI |
| 2026-04-11 | Phase 7 完成：Security 增强 | Owen + AI |
| 2026-04-10 | Phase 6 完成：API Layer 实现 | Owen + AI |
| 2026-04-09 | Phase 5 完成：Security Layer 实现 | Owen + AI |
| 2026-04-08 | Phase 4 完成：Transport Layer 修复 | Owen + AI |
| 2026-04-07 | Phase 3 完成：Discovery Layer 修复 | Owen + AI |
| 2026-04-06 | Phase 2 完成：Identity Layer 修复 | Owen + AI |
| 2026-04-05 | Phase 1 完成：类型系统修复 | Owen + AI |
| 2026-03-30 | 项目初始化，15 阶段流水线设计 | Owen + AI |