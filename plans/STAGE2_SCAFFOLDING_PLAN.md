# 阶段 2: PROJECT_SCAFFOLDING 实施计划

> **创建时间:** 2026-03-30 | **状态:** 待实施

---

## 1. 目标

根据 [`docs/ARCHITECTURE.md`](../docs/ARCHITECTURE.md) 的四层架构设计，初始化 Nexa-net 的 `src/` 目录结构、依赖管理和构建系统。

---

## 2. 项目结构设计

基于四层架构模型，设计如下目录结构：

```
nexa-net/
├── Cargo.toml                    # Rust 工作空间根配置
├── Cargo.lock
├── .gitignore
├── README.md
├── PROGRESS.md
│
├── docs/                         # 设计文档 (已完成)
│   ├── ARCHITECTURE.md
│   ├── IDENTITY_LAYER.md
│   ├── DISCOVERY_LAYER.md
│   ├── TRANSPORT_LAYER.md
│   ├── ECONOMY_LAYER.md
│   └── ...
│
├── proto/                        # Protobuf 协议定义
│   ├── nexa_message.proto        # 通用消息封装
│   ├── identity.proto            # 身份协议消息
│   ├── discovery.proto           # 发现协议消息
│   ├── transport.proto           # 传输协议消息
│   └── economy.proto             # 经济协议消息
│
├── src/                          # Rust 核心代码
│   ├── lib.rs                    # 库入口
│   │
│   ├── identity/                 # Layer 1: 身份与零信任网络层
│   │   ├── mod.rs
│   │   ├── did.rs                # DID 生成与解析
│   │   ├── did_document.rs       # DID Document 管理
│   │   ├── key_management.rs     # 密钥生成、存储、轮换
│   │   ├── mTLS.rs               # 双向 TLS 认证
│   │   ├── credential.rs         # 可验证凭证 (VC)
│   │   └── trust_anchor.rs       # 信任锚与治理
│   │
│   ├── discovery/                # Layer 2: 语义发现与能力路由层
│   │   ├── mod.rs
│   │   ├── capability.rs         # 能力 Schema 定义
│   │   ├── vectorizer.rs         # 语义向量化
│   │   ├── semantic_dht.rs       # 分布式语义哈希表
│   │   ├── router.rs             # 语义路由算法
│   │   └── node_status.rs        # 节点状态管理
│   │
│   ├── transport/                # Layer 3: 传输与协商协议层
│   │   ├── mod.rs
│   │   ├── negotiator.rs         # 动态协议协商
│   │   ├── rpc.rs                # 流式 RPC 引擎
│   │   ├── serialization.rs      # Protobuf/FlatBuffers 序列化
│   │   ├── connection.rs         # 连接池管理
│   │   └── error_handler.rs      # 错误处理与重试
│   │
│   ├── economy/                  # Layer 4: 资源管理与微交易层
│   │   ├── mod.rs
│   │   ├── token.rs              # Nexa-Token 定义
│   │   ├── channel.rs            # 状态通道管理
│   │   ├── receipt.rs            # 微交易收据
│   │   ├── budget.rs             # 预算控制
│   │   └── settlement.rs         # 结算引擎
│   │
│   ├── protocol/                 # 协议消息定义 (生成自 proto/)
│   │   └── mod.rs
│   │
│   ├── proxy/                    # Nexa-Proxy 核心守护进程
│   │   ├── mod.rs
│   │   ├── server.rs             # 本地 API 服务器
│   │   ├── client.rs             # 网络客户端
│   │   ├── config.rs             # 配置管理
│   │   └── main.rs               # 入口点
│   │
│   ├── nexa/                     # Nexa 语言集成
│   │   ├── mod.rs
│   │   ├── runtime.rs            # AVM 运行时接口
│   │   ├── dag_executor.rs       # DAG 执行器
│   │   └── network_bridge.rs     # 网络桥接
│   │
│   └── api/                      # SDK 接口
│       ├── mod.rs
│       ├── rest.rs               # REST API
│       ├── grpc.rs               # gRPC 服务
│       └── sdk.rs                # SDK 封装
│
├── sdk/                          # 多语言 SDK
│   ├── python/
│   │   ├── pyproject.toml
│   │   └── nexa_net/
│   │       ├── __init__.py
│   │       ├── client.py
│   │       └── models.py
│   │
│   └── typescript/
│       ├── package.json
│       ├── tsconfig.json
│       └── src/
│           ├── index.ts
│           ├── client.ts
│           └── models.ts
│
├── tests/                        # 测试目录
│   ├── integration/
│   └── fixtures/
│
├── examples/                     # 示例代码
│   ├── basic_call.rs
│   └── multi_agent.rs
│
├── deployments/                  # 部署配置
│   ├── docker/
│   │   ├── Dockerfile
│   │   └── docker-compose.yml
│   └── kubernetes/
│       └── nexa-proxy.yaml
│
└── scripts/                      # 构建脚本
    ├── build_proto.sh            # Protobuf 代码生成
    └── release.sh                # 发布脚本
```

---

## 3. 依赖规划

### 3.1 Rust 核心依赖

```toml
# Cargo.toml (工作空间根)
[workspace]
members = [
    "src",
    "sdk/python/native",
]

[workspace.dependencies]
# 异步运行时
tokio = { version = "1.35", features = ["full"] }
async-trait = "0.1"

# 序列化
prost = "0.12"                    # Protobuf
flatbuffers = "23.5"              # FlatBuffers
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# 加密与安全
ed25519-dalek = "2.1"             # Ed25519 签名
x25519-dalek = "2.0"              # X25519 密钥交换
ring = "0.17"                     # TLS/加密原语
rand = "0.8"

# 网络与协议
tonic = "0.10"                    # gRPC
tonic-build = "0.10"
hyper = { version = "1.0", features = ["full"] }
tower = "0.4"

# 向量与语义
hnsw = "0.11"                     # HNSW 向量索引
ndarray = "0.15"

# 存储
rocksdb = "0.21"                  # 本地 KV 存储
sled = "0.34"                     # 嵌入式数据库

# 日志与监控
tracing = "0.1"
tracing-subscriber = "0.3"
metrics = "0.21"

# 错误处理
thiserror = "1.0"
anyhow = "1.0"

# 测试
criterion = "0.5"                 # 性能基准测试
proptest = "1.4"                  # 属性测试
```

### 3.2 Python SDK 依赖

```toml
# sdk/python/pyproject.toml
[project]
name = "nexa-net"
version = "0.1.0"
dependencies = [
    "grpcio>=1.60.0",
    "grpcio-tools>=1.60.0",
    "pydantic>=2.0.0",
    "httpx>=0.25.0",
    "cryptography>=41.0.0",
]
```

### 3.3 TypeScript SDK 依赖

```json
// sdk/typescript/package.json
{
  "name": "@nexa-net/sdk",
  "version": "0.1.0",
  "dependencies": {
    "@grpc/grpc-js": "^1.9.0",
    "protobufjs": "^7.2.0",
    "axios": "^1.6.0"
  }
}
```

---

## 4. 构建系统

### 4.1 Protobuf 代码生成

```bash
#!/bin/bash
# scripts/build_proto.sh

set -e

echo "Generating Rust code from Protobuf..."
protoc --rust_out=src/protocol \
       --tonic_out=src/protocol \
       proto/*.proto

echo "Generating Python code from Protobuf..."
python -m grpc_tools.protoc \
       --python_out=sdk/python/nexa_net/generated \
       --grpc_python_out=sdk/python/nexa_net/generated \
       proto/*.proto

echo "Generating TypeScript code from Protobuf..."
protoc --ts_out=sdk/typescript/src/generated \
       proto/*.proto

echo "Protobuf code generation complete!"
```

### 4.2 Makefile

```makefile
.PHONY: all build test clean proto

all: proto build

proto:
	@./scripts/build_proto.sh

build:
	cargo build --release

test:
	cargo test --all

clean:
	cargo clean
	rm -rf src/protocol/generated/*
	rm -rf sdk/python/nexa_net/generated/*
	rm -rf sdk/typescript/src/generated/*

dev:
	cargo run --bin nexa-proxy

docker:
	docker build -t nexa-net/proxy:latest -f deployments/docker/Dockerfile .
```

---

## 5. 实施步骤

### 步骤 1: 创建基础目录结构
```bash
mkdir -p proto src/{identity,discovery,transport,economy,protocol,proxy,nexa,api}
mkdir -p sdk/python/nexa_net sdk/typescript/src
mkdir -p tests/{integration,fixtures}
mkdir -p examples deployments/{docker,kubernetes} scripts
```

### 步骤 2: 初始化 Cargo 工作空间
```bash
cargo init --lib
```

### 步骤 3: 创建 Protobuf 定义文件
按照 PROTOCOL_SPEC.md 创建 proto/*.proto 文件

### 步骤 4: 配置构建脚本
创建 Makefile 和 scripts/build_proto.sh

### 步骤 5: 验证构建
```bash
make proto
cargo check
```

---

## 6. 验收标准

- [ ] `src/` 目录结构符合四层架构设计
- [ ] Cargo.toml 配置正确，依赖可解析
- [ ] Protobuf 文件创建完成，可生成 Rust 代码
- [ ] `cargo check` 通过无错误
- [ ] `cargo build` 成功编译

---

## 7. 下一步

完成 PROJECT_SCAFFOLDING 后，进入 **阶段 3: IDENTITY_IMPLEMENT**，实现 Layer 1 身份与零信任网络层。

---

*此计划由 Nexa-net 工程流水线生成，需切换到 Code 模式执行实施。*