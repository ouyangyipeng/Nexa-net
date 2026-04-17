# Phase 10 详细执行计划：集成测试与 E2E 测试

## 现状分析

### 已有资产
- **5 个 E2E 场景已实现** (`tests/e2e_test.rs`, 692 行) — 但全部是 **内存级** 测试，直接调用 Rust API，未经过 HTTP 层
- **TestAgent / TestEnvironment** 已在 `tests/common/mod.rs` 实现
- **REST API** (`src/api/rest.rs`) 已有完整端点: `/v1/call`, `/v1/register`, `/v1/discover`, `/v1/channels`, `/v1/balance`, `/v1/status`, `/v1/health`
- **ProxyState** (`src/proxy/server.rs`) 已将所有核心组件组装在一起
- **469 个单元/集成测试** 已全部通过

### 缺口
1. **无 HTTP 级 E2E 测试** — 现有测试绕过 REST API，直接操作内存结构
2. **TestProxy 未实现** — 计划要求"启动本地 REST+gRPC"的轻量测试 Proxy
3. **MockNetwork 未实现** — 计划要求 `tokio::net::TcpListener` 本地回环
4. **测试重复** — `tests/integration/e2e_test.rs` 是 `tests/e2e_test.rs` 的旧版副本（仅 4 个基础测试，194 行），应移除
5. **gRPC 服务未完全实现** — 仅 health check，故 Phase 10 聚焦 REST API E2E

---

## 执行步骤

### Step 1: 清理重复测试

**操作**: 删除 `tests/integration/e2e_test.rs` 并更新 `tests/integration.rs` 移除 `mod e2e_test`

**理由**: 此文件是 `tests/e2e_test.rs` 的旧版副本，仅含 4 个基础测试，与主 E2E 文件功能重叠。保留 `tests/integration/channel_test.rs` 和 `tests/integration/discovery_test.rs` 因为它们测试不同维度。

**文件变更**:
- 删除 `tests/integration/e2e_test.rs`
- 修改 `tests/integration.rs`: 移除 `mod e2e_test;`

---

### Step 2: 实现 TestProxy

**位置**: `tests/common/mod.rs` 中新增 `TestProxy` 结构体

**设计**:
```rust
pub struct TestProxy {
    /// ProxyState (核心组件集合)
    state: Arc<ProxyState>,
    /// REST API 服务器地址
    address: String,
    /// tokio 任务 handle (用于 shutdown)
    server_handle: JoinHandle<Result<()>>,
}
```

**实现要点**:
1. `TestProxy::new()` — 创建 `ProxyState` 默认实例，使用 `TestEnvironment` 的 config
2. `TestProxy::start()` — 在 `127.0.0.1:0` 绑定 TCP listener（随机端口），启动 axum REST server
3. `TestProxy::shutdown()` — 通过 `tokio::sync::mpsc` 发送 shutdown 信号
4. 随机端口获取: `TcpListener::bind("127.0.0.1:0")` → `local_addr()` → 提取端口
5. 所有操作异步，但提供 `block_on` 包装用于同步测试

**辅助方法**:
- `TestProxy::endpoint()` → 返回 `http://127.0.0.1:{port}`
- `TestProxy::register_capability(schema)` → 内部直接操作 `ProxyState.registry`
- `TestProxy::discover(intent, max_candidates)` → HTTP POST `/v1/discover`
- `TestProxy::call(intent, data, budget)` → HTTP POST `/v1/call`
- `TestProxy::open_channel(payer, payee, deposit_a, deposit_b)` → 内部操作 `ChannelManager`
- `TestProxy::health_check()` → HTTP GET `/v1/health`

---

### Step 3: 实现 MockNetwork

**位置**: `tests/common/mod.rs` 中新增 `MockNetwork`

**设计**:
```rust
pub struct MockNetwork {
    /// 可用的 TCP listener 地址列表
    endpoints: Vec<String>,
    /// 控制: 使某个 endpoint 不可用 (模拟断线)
    unavailable: HashSet<String>,
}
```

**实现要点**:
1. `MockNetwork::new(n: usize)` — 创建 n 个 `TcpListener::bind("127.0.0.1:0")` 获取随机端口地址
2. `MockNetwork::mark_unavailable(addr)` — 模拟节点断线
3. `MockNetwork::mark_available(addr)` — 模拟节点恢复
4. 用于 `test_fault_recovery` 场景: Agent B 断线 → 发现 Agent C 备选

---

### Step 4: HTTP 级 E2E 测试

**位置**: `tests/e2e_http_test.rs` (新文件)

**为什么新建文件而非扩展现有文件**: 
- 现有 `tests/e2e_test.rs` 是同步测试（`#[test]`），新文件需要异步测试（`#[tokio::test]`）
- HTTP 测试需要 TestProxy 启动/关闭生命周期，与内存测试模式不同
- 避免混合两种测试模式增加维护复杂度

#### 场景 1: HTTP 双 Agent 通信
```
TestProxy 启动 → Agent B 注册 Translation → HTTP POST /v1/discover 
→ HTTP POST /v1/call → 验证响应 → 内部验证 channel/receipt 状态
```

**关键验证**:
- REST API 序列化/反序列化正确性
- 请求经过完整 HTTP 层（JSON → struct → 处理 → struct → JSON）
- Health check 端点可访问

#### 场景 2: HTTP 多 Agent 社区
```
TestProxy 启动 → 5 个 Agent 注册 → 5 次 HTTP POST /v1/discover 
→ 验证每个 intent 返回正确的 provider → 验证并发安全性
```

**关键验证**:
- 多次 HTTP 请求无竞态条件
- 跨 Agent 路由发现正确性

#### 场景 3: HTTP 经济闭环
```
TestProxy 启动 → Agent 注册 → open_channel → 10 次 HTTP POST /v1/call
→ 验证 receipt chain → close_channel → settlement
```

**关键验证**:
- 经济操作经过 REST API 正确处理
- 余额不变量在 HTTP 级操作后仍成立

#### 场景 4: HTTP 故障恢复
```
TestProxy 启动 → Agent B 注册 → mark_unavailable → Agent C 注册备选
→ HTTP POST /v1/discover → 验证返回 Agent C 而非 B
→ HTTP POST /v1/call 到 C → 验证成功
```

#### 场景 5: HTTP 安全验证
```
TestProxy 启动 → 测试 budget exceeded 被 /v1/call 拒绝
→ 测试 rate limiting 在多次 /v1/discover 后生效
→ 验证 audit events 记录
```

---

### Step 5: 验收标准确认

- `cargo test` 全部通过（包括新 HTTP E2E 测试）
- 5 个 E2E 场景通过 REST API 层
- 无硬编码等待时间（使用随机端口 + 事件驱动）
- 全部场景 < 30 秒完成

---

### Step 6: 文档更新

- `PROGRESS.md` — 标记 Phase 10 完成
- `docs/ROADMAP.md` — 记录 Phase 10 决策和变更

---

## 文件变更清单

| 文件 | 操作 | 说明 |
|------|------|------|
| `tests/integration/e2e_test.rs` | DELETE | 旧版重复测试 |
| `tests/integration.rs` | MODIFY | 移除 `mod e2e_test` |
| `tests/common/mod.rs` | MODIFY | 新增 TestProxy, MockNetwork |
| `tests/e2e_http_test.rs` | CREATE | HTTP 级 E2E 测试 (5 个场景) |
| `PROGRESS.md` | MODIFY | 标记 Phase 10 完成 |
| `docs/ROADMAP.md` | MODIFY | 记录 Phase 10 变更 |

---

## 依赖与风险

1. **RestServer 需要暴露 shutdown 机制** — 当前 `RestServer::start()` 是无限运行，需要添加 graceful shutdown (tokio mpsc channel 或 CancellationToken)
2. **ProxyState 构造** — 需要确认 `ProxyState::new()` 或类似构造函数是否已实现；若无，需在 `src/proxy/server.rs` 中添加
3. **端口 0 绑定** — axum 0.7+ 支持 `TcpListener::bind("0.0.0.0:0")` + `local_addr()` 获取随机端口，需确认当前 axum 版本支持
4. **测试隔离** — 每个 HTTP E2E 测试需独立的 TestProxy 实例，避免状态交叉污染

---

## 架构流程图

```mermaid
graph TD
    A[TestProxy::start] --> B[TcpListener bind 127.0.0.1:0]
    B --> C[获取随机端口]
    C --> D[构建 ProxyState]
    D --> E[启动 axum REST server]
    E --> F{HTTP E2E 测试}
    
    F --> G[POST /v1/register]
    F --> H[POST /v1/discover]
    F --> I[POST /v1/call]
    F --> J[GET /v1/health]
    
    G --> K[ProxyState.registry 写入]
    H --> L[ProxyState.router 查询]
    I --> M[ProxyState.channels + budget 操作]
    J --> N[返回 200 OK]
    
    F --> O[TestProxy::shutdown]
    O --> P[发送 shutdown 信号]
    P --> Q[服务器停止]