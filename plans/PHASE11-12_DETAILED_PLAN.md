# Phase 11-12 合并执行计划

## Phase 11: 性能基准测试与优化（收尾）

### 现状
- 42 个 Criterion benchmarks 已覆盖全部 6 模块
- 所有性能目标已远超要求（路由 3200x、通道 990x、序列化 56x）
- `docs/PERFORMANCE.md` 已有 265 行完整报告
- DashMap/SIMD/Pre-allocation 优化已应用

### 仅需补充的工作

#### 11.1 REST API 端点延迟基准
在 `benches/nexa_bench.rs` 新增异步 benchmark group:
- `api/rest_health` — GET /v1/health 端点延迟
- `api/rest_discover` — POST /v1/discover 端点延迟
- `api/rest_register` — POST /v1/register 端点延迟

**实现方式**: 使用 Phase 10 的 TestProxy 模式（TcpListener 随机端口 + axum serve），在 benchmark 中启动/关闭 TestProxy。

注意: Criterion 的 async_tokio feature 已在 dev-dependencies 中配置。

#### 11.2 验证 cargo bench 全量运行
运行 `cargo bench` 并确认:
- 所有 benchmark 组运行无错误（之前有 OOM 问题 noted）
- 如遇 OOM，对受影响组降低 sample_size（`BenchmarkId::new` + `Criterion::sample_size(10)`）

#### 11.3 更新 PERFORMANCE.md
运行 bench 后将新结果（特别是 REST API 延迟数据）追加到 `docs/PERFORMANCE.md`。

---

## Phase 12: 文档同步与最终打磨

### 12.1 代码质量打磨

#### 12.1.1 Clippy 修复
运行 `cargo clippy -- -D warnings` 并修复所有 warnings。

已知 warning 来源:
- `#[allow(dead_code)]` 标注（约 6 处）— 评估是否需要移除或补充实现
- unused imports/variables — 移除或标注
- 死代码 — 移除或补充 `///` 文档说明为 intentionally unused

#### 12.1.2 TODO/FIXME 清理
36 个 TODO 注释分布:
- **Storage backends** (postgres.rs, redis.rs): "TODO: Implement in Phase 1" — 这些是 feature-gated stubs，改为更准确的注释如 "Stub: requires database connection"
- **SDK streaming** (sdk.rs): "TODO: Implement streaming" — 标注为 "Future: requires WebSocket/gRPC streaming support"
- **Transport serialization** (serialization.rs): Protobuf/FlatBuffers fallback — 注释已说明
- **Proxy client** (client.rs): placeholder — 注释说明
- **Discovery router** (router.rs): latency placeholder — 注释说明
- **Nexa language** (runtime.rs, network_bridge.rs, dag_executor.rs): placeholder — 注释说明

处理策略:
- 对 stub/placeholder TODO: 改为 `// NOTE: ...` 注释说明当前状态和未来计划
- 对真正需要实现的 TODO: 保留但补充上下文
- 删除已完成的 TODO

#### 12.1.3 cargo fmt
运行 `cargo fmt -- --check` 确保格式一致。

### 12.2 文档更新

需要更新的文档:

| 文件 | 更新内容 |
|------|---------|
| `README.md` | 项目描述、快速开始、架构概览、测试统计 |
| `docs/ARCHITECTURE.md` | 对齐 ProxyState 共享 registry bug fix、SemanticRouter Arc<RwLock> 重构 |
| `docs/API_REFERENCE.md` | 对齐 REST API 端点（确认 /v1/ 前缀）、新增 TestProxy |
| `docs/DEVELOPER_GUIDE.md` | 更新开发指南、测试说明（包括 HTTP E2E） |
| `docs/ROADMAP.md` | 更新进度、Phase 10 bug fix 记录 |
| `PROGRESS.md` | 更新为最终状态 |

### 12.3 ROADMAP.md 更新
根目录 `ROADMAP.md` 记录:
- Phase 10 的关键 bug fix（ProxyState registry/router 不共享实例）
- Phase 10 新增的 HTTP E2E 测试基础设施
- 错误学习：registry 和 router 使用不同 registry 实例导致 register 写入对 discover 不可见

---

## 文件变更清单

| 文件 | 操作 | Phase |
|------|------|-------|
| `benches/nexa_bench.rs` | MODIFY | 11 — 新增 REST API benchmark group |
| `docs/PERFORMANCE.md` | MODIFY | 11 — 追加 REST API 延迟数据 |
| `src/**/*.rs` | MODIFY | 12 — 修复 clippy warnings + TODO 清理 |
| `README.md` | MODIFY | 12 — 更新项目描述 |
| `docs/ARCHITECTURE.md` | MODIFY | 12 — 对齐 ProxyState bug fix |
| `docs/API_REFERENCE.md` | MODIFY | 12 — 对齐 REST API |
| `docs/DEVELOPER_GUIDE.md` | MODIFY | 12 — 更新开发指南 |
| `docs/ROADMAP.md` | MODIFY | 12 — Phase 10 记录 + bug fix |
| `ROADMAP.md` | MODIFY | 12 — 根目录路线图更新 |
| `PROGRESS.md` | MODIFY | 12 — 最终状态 |

---

## 执行顺序

1. **Phase 11** → 新增 REST API benchmarks + cargo bench 验证 + PERFORMANCE.md 更新
2. **Phase 12** → clippy 修复 → TODO 清理 → cargo fmt → 文档同步 → ROADMAP.md
3. 最终验收: `cargo test` + `cargo clippy -- -D warnings` + `cargo fmt -- --check` + `cargo bench` 全部通过