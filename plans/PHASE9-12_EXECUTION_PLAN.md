# Phase 9-12 执行计划

## 当前状态

- Phase 1-8 已完成，230 lib 测试 + 31 集成测试全部通过
- cargo check --features storage-rocksdb 通过
- examples 编译通过
- 仍有 28 个 warnings（unused imports/variables/dead code）

## Phase 9: 全面单元测试

### 目标
每个模块 ≥80% 覆盖率，proptest property-based 测试 1000 次迭代无失败

### 执行步骤

#### 9.1 添加 proptest 测试模块
在 `src/` 各模块的 `mod tests` 中添加 property-based 测试：

- **Identity proptest** (`src/identity/`):
  - `did.rs`: DID 字符串格式合法性 — `did:nexa:*` 格式验证
  - `did_document.rs`: DID Document 序列化 round-trip (JSON → struct → JSON)
  - `credential.rs`: VC 签名→验证 闭环 + 过期拒绝
  - `key_management.rs`: KeyPair generate → sign → verify round-trip + zeroize 验证

- **Discovery proptest** (`src/discovery/`):
  - `capability.rs`: CapabilitySchema 注册→查询→标签搜索的一致性
  - `semantic_dht.rs`: HNSW insert → search 回召率验证（随机向量）
  - `router.rs`: 路由权重计算 → normalize → score 排序一致性

- **Transport proptest** (`src/transport/`):
  - `frame.rs`: 帧编码→解码 round-trip (任意 payload)
  - `stream.rs`: 流状态机转换合法性（不会跳过中间状态）
  - `serialization.rs`: serialize → compress → decompress → deserialize round-trip

- **Economy proptest** (`src/economy/`):
  - `channel.rs`: 通道余额不变量 total = balance_a + balance_b（任意合法操作序列）
  - `channel.rs`: 通道状态转换合法性（Open→Active→Closing→Closed 不可跳跃）
  - `receipt.rs`: 收据签名+哈希链验证 round-trip
  - `budget.rs`: 预算超限自动终止验证

- **Security proptest** (`src/security/`):
  - `secure_storage.rs`: AES-256-GCM encrypt → decrypt round-trip（任意 key data）
  - `rate_limit.rs`: 速率限制计数器一致性（任意请求序列）

#### 9.2 补充边界/错误测试
对每个公开方法补充至少 1 个正向 + 1 个边界/错误测试：
- Identity: DID parse 无效格式、VC 过期验证、空 DID Document
- Discovery: 空 registry 搜索、0 向量搜索、max capabilities 超限
- Transport: 空 payload 帧、过大帧、无效压缩数据
- Economy: 0 余额通道、负数金额拒绝、空收据链
- Security: 空 key store、无 encryption key 时的 encrypt 拒绝

#### 9.3 验收标准
- `cargo test` 全部通过
- proptest 1000 次迭代无失败
- 无 `#[ignore]` 测试

---

## Phase 10: 集成测试与 E2E 测试

### 目标
5 个 E2E 场景全部通过，无硬编码等待时间

### 执行步骤

#### 10.1 重构现有集成测试
现有 `tests/integration/` 目录下的测试需要更新为使用新的 API：

- `channel_test.rs`: 更新为使用 `ChannelManager` 的新接口
- `discovery_test.rs`: 更新为使用 `SemanticRouter` + `HnswIndex` 的新接口
- `e2e_test.rs`: 更新为使用 `IdentityKeys` + `SecurityManager`

#### 10.2 实现 5 个 E2E 场景

**Scenario 1: 单机双Agent通信**
```
Agent A (translate intent) → discover → Agent B
验证: 路由发现 + RPC 调用 + 收据生成 + 通道结算
```

**Scenario 2: 多Agent社区 (5个Agent)**
```
5个Agent注册不同能力 → 交叉调用 A→B→C→D→E→A
验证: 路由正确性 + 并发安全性 + 预算控制
```

**Scenario 3: 故障恢复**
```
Agent B 断线 → 重试 → 发现备选 Agent C → 完成调用
验证: 错误重试 + 降级路由
```

**Scenario 4: 经济闭环**
```
A给B开通道 → 10次调用 → 10张收据 → 关闭通道 → 结算
验证: 余额计算 + 收据链完整性 + 结算正确性
```

**Scenario 5: 安全验证**
```
未签名请求 → 拒绝
伪造 VC → 拒绝
预算超限 → 终止
验证: 零信任架构有效性
```

#### 10.3 创建 TestProxy 工具
在 `tests/common/mod.rs` 中创建:
- `TestProxy`: 轻量级测试用 Proxy 实例（启动本地 REST+gRPC）
- `TestAgent`: 生成 IdentityKeys + 注册 Capability 的测试 Agent
- `MockNetwork`: 使用 `tokio::net::TcpListener` 本地回环

#### 10.4 验收标准
- 5 个 E2E 场景全部通过
- 无硬编码等待时间
- 测试可在 < 30秒 内完成全部场景

---

## Phase 11: 性能基准测试与优化

### 目标
工业级性能达标，所有基准运行无错误

### 执行步骤

#### 11.1 扩展 Criterion 基准测试
在 `benches/nexa_bench.rs` 中新增:

- 帧编码/解码吞吐量 (Frame::encode → decode round-trip)
- 序列化+压缩 pipeline 吞吐量 (JSON → serialize → LZ4/Zstd → decompress → deserialize)
- HNSW 索引构建 + 搜索延迟 (insert 1K vectors → search top-5)
- 通道更新+签名 TPS (Channel::update → sign receipt)
- 收据生成+验证 TPS (MicroReceipt::new → sign → verify)
- REST API 端点延迟 (使用 axum::test helper)
- AES-256-GCM 加密/解密吞吐量
- 速率限制检查吞吐量

#### 11.2 性能优化
按优先级排序:

1. **锁优化**: `std::sync::RwLock` → `parking_lot::RwLock` / `dashmap` (已在 Cargo.toml)
2. **批量操作**: `ReceiptChain::batch_create_receipts()` / `MemoryStore::batch_store_receipts()`
3. **对象池**: 连接/帧对象复用减少分配
4. **SIMD**: `cosine_similarity` 使用 SIMD 加速 (通过 `std::simd` 或手动 unroll)
5. **零拷贝**: `bytes::Bytes` 替代 `Vec<u8>` 在帧 payload 中

#### 11.3 生成性能报告
运行 `cargo bench` → 结果记录到 `docs/PERFORMANCE.md`

#### 11.4 验收标准
- 所有基准测试运行无错误
- 路由延迟 < 100ms
- 通道更新 TPS > 10K
- 基准报告记录到 docs/PERFORMANCE.md

---

## Phase 12: 文档同步与最终打磨

### 目标
代码质量达标，文档与代码同步

### 执行步骤

#### 12.1 代码质量打磨
- `cargo clippy -- -D warnings` 修复所有 28 个 warnings
- `cargo fmt` 格式统一
- 清理所有 TODO/FIXME/hack 注释
- 死代码检测和清理
- 所有公开 API 补充 `///` 文档注释

#### 12.2 文档更新
- `README.md` — 更新项目描述、快速开始、架构概览
- `docs/ARCHITECTURE.md` — 对齐实际实现
- `docs/API_REFERENCE.md` — 对齐 REST/gRPC 端点
- `docs/DEVELOPER_GUIDE.md` — 更新开发指南
- `docs/ROADMAP.md` — 更新进度和决策记录
- `PROGRESS.md` — 更新进度追踪
- `plans/REFACTORING_PLAN.md` — 标记所有 Phase 完成

#### 12.3 生成 ROADMAP.md
根目录新增 `ROADMAP.md`，记录:
- 设计思路和决策过程
- 每次迭代的变更点
- 错误学习和改进措施

#### 12.4 验收标准
- `cargo clippy -- -D warnings` 无警告
- `cargo fmt --check` 通过
- 所有文档与代码同步
- ROADMAP.md 记录重构决策和错误学习

---

## 委托顺序

1. **Phase 9** → code 子任务（proptest + 补充测试）
2. **Phase 10** → code 子任务（E2E 场景 + TestProxy）
3. **Phase 11** → code 子任务（基准扩展 + 优化）
4. **Phase 12** → code 子任务（clippy + 文档同步）

每个 Phase 完成后验证 `cargo test` 全部通过再继续下一个。