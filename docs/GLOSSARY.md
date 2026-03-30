# Nexa-net 术语表

> **版本:** v1.0.0-draft | **最后更新:** 2026-03-30

## 目录

- [A](#a)
- [B](#b)
- [C](#c)
- [D](#d)
- [E](#e)
- [F](#f)
- [G](#g)
- [H](#h)
- [I](#i)
- [K](#k)
- [L](#l)
- [M](#m)
- [N](#n)
- [O](#o)
- [P](#p)
- [R](#r)
- [S](#s)
- [T](#t)
- [U](#u)
- [V](#v)
- [W](#w)
- [Z](#z)
- [缩写表](#缩写表)

---

## A

### Agent（智能体）

**定义：** 能够自主执行任务、做出决策并与环境交互的软件实体。在 Nexa-net 中，Agent 是网络的基本参与者。

**相关术语：** Autonomous Agent, LLM Agent

**示例：** 基于 LangChain 构建的文档处理 Agent。

---

### ANN (Approximate Nearest Neighbor)

**定义：** 近似最近邻搜索，一种在高维空间中快速找到与查询点最相似点的算法。

**相关术语：** HNSW, IVF, Vector Search

**应用场景：** Nexa-net 语义路由中的向量相似度搜索。

---

### Authentication（认证）

**定义：** 验证实体身份的过程。在 Nexa-net 中通过 DID 和 mTLS 实现。

**相关术语：** Authorization, mTLS, DID

---

### Authorization（授权）

**定义：** 确定已认证实体可以访问哪些资源的过程。在 Nexa-net 中通过 VC 实现。

**相关术语：** Authentication, VC, Permission

---

### Autonomous Agent（自治智能体）

**定义：** 能够在无需人类干预的情况下自主运行和决策的 Agent。

**相关术语：** Agent, LLM Agent

---

## B

### Balance（余额）

**定义：** Agent 在 Nexa-net 中的 Token 余额，用于支付服务调用费用。

**相关术语：** Token, Channel, Deposit

---

### Binary Protocol（二进制协议）

**定义：** 使用二进制格式而非文本格式传输数据的通信协议。Nexa-net 默认使用 Protobuf。

**相关术语：** Protobuf, FlatBuffers, Serialization

---

### Budget（预算）

**定义：** 单次服务调用允许的最大费用上限。

**相关术语：** Cost, Token, Micro-transaction

---

## C

### Capability（能力）

**定义：** Agent 提供的可被其他 Agent 调用的服务或功能。

**相关术语：** Endpoint, Service, Schema

---

### Capability Schema（能力清单）

**定义：** 描述 Agent 能力的结构化文档，包含端点定义、输入输出 Schema、成本模型等。

**相关术语：** Endpoint, Input Schema, Output Schema, Cost Model

---

### Certificate（证书）

**定义：** 用于 TLS 加密通信的数字证书。在 Nexa-net 中使用自签名证书，通过 DID 验证。

**相关术语：** TLS, mTLS, PKI

---

### Channel（通道）

**定义：** 两个 Agent 之间建立的支付通道，用于高频微交易。

**相关术语：** State Channel, Payment Channel, Micro-transaction

---

### Cosine Similarity（余弦相似度）

**定义：** 衡量两个向量之间相似度的指标，值为两个向量夹角的余弦值。

**公式：** $\text{Similarity} = \frac{A \cdot B}{||A|| \times ||B||}$

**应用场景：** Nexa-net 语义路由中的意图匹配。

---

## D

### Data Plane（数据平面）

**定义：** Nexa-Proxy 中负责处理实际数据传输的组件。

**相关术语：** Control Plane, Nexa-Proxy

---

### DHT (Distributed Hash Table)

**定义：** 分布式哈希表，一种去中心化的键值存储系统。

**应用场景：** Nexa-net 中存储能力索引和路由信息。

**相关术语：** Semantic DHT, Kademlia

---

### DID (Decentralized Identifier)

**定义：** 去中心化标识符，一种不依赖中心化机构的身份标识方式。

**格式：** `did:nexa:<identifier>`

**相关术语：** DID Document, Verifiable Credential

---

### DID Document（DID 文档）

**定义：** 描述 DID 的 JSON-LD 文档，包含公钥、验证方法和服务端点等信息。

**相关术语：** DID, Verification Method

---

### Discovery Layer（发现层）

**定义：** Nexa-net 四层架构中的第二层，负责服务发现和语义路由。

**相关术语：** Semantic Routing, Capability Registration

---

### Deposit（保证金）

**定义：** 开启支付通道时锁定的 Token 数量。

**相关术语：** Channel, Balance, State Channel

---

## E

### Edge Node（边缘节点）

**定义：** 运行 Agent 和 Nexa-Proxy 的节点，与 Supernode 相对。

**相关术语：** Supernode, Nexa-Proxy, Agent

---

### Embedding（嵌入向量）

**定义：** 将文本或其他数据转换为高维向量的过程。

**相关术语：** Vector, Semantic Vector, ANN

---

### Endpoint（端点）

**定义：** Agent 提供的具体服务入口点。

**相关术语：** Capability, Service, API

---

### Economy Layer（经济层）

**定义：** Nexa-net 四层架构中的第四层，负责微交易和资源管理。

**相关术语：** Token, Channel, Micro-transaction

---

## F

### FlatBuffers

**定义：** 一种高效的二进制序列化格式，支持零拷贝访问。

**相关术语：** Protobuf, Serialization, Zero-copy

---

### Flow Control（流量控制）

**定义：** 控制数据传输速率以防止接收方过载的机制。

**相关术语：** Backpressure, Window Size

---

## G

### gRPC

**定义：** Google 开发的高性能 RPC 框架，基于 HTTP/2 和 Protobuf。

**相关术语：** RPC, Protobuf, HTTP/2

---

## H

### Handshake（握手）

**定义：** 建立连接时双方交换信息并协商参数的过程。

**相关术语：** SYN-NEXA, ACK-SCHEMA, Negotiation

---

### Heartbeat（心跳）

**定义：** 定期发送的小型消息，用于检测节点是否在线。

**相关术语：** Health Check, Keep-alive

---

### HNSW (Hierarchical Navigable Small World)

**定义：** 一种高效的近似最近邻搜索算法和数据结构。

**相关术语：** ANN, Vector Index, Similarity Search

---

## I

### Identity Layer（身份层）

**定义：** Nexa-net 四层架构中的第一层，负责身份认证和权限管理。

**相关术语：** DID, mTLS, Verifiable Credential

---

### Intent（意图）

**定义：** Agent 发出的服务请求描述，用于语义路由匹配。

**示例：** "translate English PDF to Chinese"

**相关术语：** Semantic Routing, Capability

---

## K

### Key Agreement（密钥协商）

**定义：** 双方在不安全通道上协商出共享密钥的密码学协议。

**相关术语：** X25519, Diffie-Hellman, ECDH

---

## L

### Layer 2（二层网络）

**定义：** 构建在主链之上的扩展协议，用于提高交易吞吐量和降低成本。

**相关术语：** State Channel, Payment Channel

---

### mTLS (Mutual TLS)

**定义：** 双向 TLS 认证，客户端和服务器都需要出示证书。

**相关术语：** TLS, Certificate, Authentication

---

## M

### M2M (Machine-to-Machine)

**定义：** 机器对机器通信，无需人类干预的自动化通信模式。

**相关术语：** Agent, Autonomous

---

### Micro-Receipt（微交易收据）

**定义：** 记录单次服务调用费用和结果的签名凭证。

**相关术语：** Receipt, Settlement, Channel

---

### Micro-transaction（微交易）

**定义：** 金额很小的交易，通常在几分钱到几块钱之间。

**相关术语：** Token, Channel, Settlement

---

### Multiplexing（多路复用）

**定义：** 在单个连接上同时传输多个独立数据流的技术。

**相关术语：** Stream, HTTP/2

---

## N

### NAT Traversal（NAT 穿透）

**定义：** 在 NAT（网络地址转换）环境下建立 P2P 连接的技术。

**相关术语：** STUN, TURN, WebRTC

---

### Nexa-DID

**定义：** Nexa-net 的 DID 方法实现，基于 Ed25519 或 Secp256k1 密钥。

**格式：** `did:nexa:<public-key-hash>`

**相关术语：** DID, Ed25519

---

### Nexa-Proxy

**定义：** Nexa-net 的核心组件，作为 Agent 的 Sidecar 代理处理网络通信。

**相关术语：** Sidecar, Proxy, Data Plane, Control Plane

---

### Nexa-Token (NEXA)

**定义：** Nexa-net 的原生代币，用于支付服务调用费用。

**相关术语：** Token, Economy Layer

---

## O

### OpenAPI

**定义：** 一种描述 REST API 的规范格式，Nexa-net 能力 Schema 基于其扩展。

**相关术语：** API, Schema, Swagger

---

## P

### P2P (Peer-to-Peer)

**定义：** 对等网络，节点之间直接通信，无需中心服务器。

**相关术语：** DHT, Decentralization

---

### Payment Channel（支付通道）

**定义：** 允许双方进行多次链下支付的二层协议。

**相关术语：** State Channel, Channel, Layer 2

---

### Protobuf (Protocol Buffers)

**定义：** Google 开发的高效二进制序列化格式。

**相关术语：** Serialization, FlatBuffers, gRPC

---

## R

### Rate Limit（速率限制）

**定义：** 限制单位时间内请求数量的机制。

**相关术语：** Throttling, Quota

---

### Receipt（收据）

**定义：** 记录交易或服务调用的凭证。

**相关术语：** Micro-Receipt, Settlement

---

### Routing（路由）

**定义：** 确定消息从源到目的地的路径的过程。

**相关术语：** Semantic Routing, Intent

---

### RPC (Remote Procedure Call)

**定义：** 远程过程调用，允许像调用本地函数一样调用远程服务。

**相关术语：** gRPC, Client, Server

---

## S

### Schema（模式）

**定义：** 定义数据结构和验证规则的形式化描述。

**相关术语：** JSON Schema, Protobuf Schema

---

### Semantic DHT（语义分布式哈希表）

**定义：** 支持基于语义相似度查询的分布式哈希表。

**相关术语：** DHT, Vector Index, ANN

---

### Semantic Routing（语义路由）

**定义：** 基于意图语义而非精确地址的服务发现和路由机制。

**相关术语：** Intent, Vector, Similarity

---

### Serialization（序列化）

**定义：** 将数据结构转换为可存储或传输的格式的过程。

**相关术语：** Protobuf, FlatBuffers, JSON

---

### Settlement（结算）

**定义：** 将通道内的净余额提交到全局账本的过程。

**相关术语：** Channel, Micro-Receipt, Ledger

---

### Sidecar Proxy（边车代理）

**定义：** 与主应用部署在同一主机上的代理进程，处理网络通信。

**相关术语：** Nexa-Proxy, Data Plane, Control Plane

---

### State Channel（状态通道）

**定义：** 允许双方进行多次链下状态更新的二层协议。

**相关术语：** Payment Channel, Channel, Layer 2

---

### STUN (Session Traversal Utilities for NAT)

**定义：** 用于 NAT 穿透的协议，帮助客户端发现自己的公网地址。

**相关术语：** NAT Traversal, TURN, WebRTC

---

### Supernode（超级节点）

**定义：** Nexa-net 网络中的高可用节点，负责路由表维护和 NAT 穿透。

**相关术语：** Registry, Relay, DHT

---

## T

### Threshold（阈值）

**定义：** 语义路由中相似度的最低要求值。

**相关术语：** Similarity, Semantic Routing

---

### Token（代币）

**定义：** Nexa-net 中的价值载体，用于支付服务费用。

**相关术语：** Nexa-Token, Economy Layer

---

### Transport Layer（传输层）

**定义：** Nexa-net 四层架构中的第三层，负责数据传输和协议协商。

**相关术语：** RPC, mTLS, Serialization

---

### TURN (Traversal Using Relays around NAT)

**定义：** 当 NAT 穿透失败时，通过中继服务器转发流量的协议。

**相关术语：** STUN, NAT Traversal, Relay

---

## U

### Unary RPC（一元 RPC）

**定义：** 单次请求-响应模式的 RPC 调用。

**相关术语：** Streaming RPC, RPC

---

## V

### VC (Verifiable Credential)

**定义：** 可验证凭证，一种密码学签名的数字凭证。

**相关术语：** DID, Issuer, Holder, Verifier

---

### Vector（向量）

**定义：** 高维空间中的数值数组，用于表示语义信息。

**相关术语：** Embedding, Semantic Vector

---

### Verifier（验证者）

**定义：** 验证 VC 真实性和有效性的实体。

**相关术语：** VC, Issuer, Holder

---

## W

### WebRTC

**定义：** 支持浏览器和应用程序进行实时通信的协议和 API。

**应用场景：** Nexa-net 中 P2P 直连通信。

**相关术语：** P2P, NAT Traversal, Data Channel

---

## Z

### Zero-copy（零拷贝）

**定义：** 一种优化技术，避免数据在内存中不必要的复制。

**相关术语：** FlatBuffers, Performance

---

### Zero-Trust（零信任）

**定义：** 一种安全模型，不信任任何预设关系，每次访问都需要验证。

**相关术语：** mTLS, Authentication, Authorization

---

## Nexa 语言术语

本节收录与 Nexa 语言集成相关的术语定义。

### AVM (Agent Virtual Machine)

**定义：** Nexa v1.0 引入的 Rust 高性能执行引擎，用于运行 Nexa 编译后的字节码。

**相关术语：** WASM Sandbox, Bytecode, Runtime

**特性：**
- 编译时间 ~5ms（对比 Python 转译 ~100ms）
- 启动时间 ~10ms（对比 Python ~500ms）
- 内存占用 ~10MB（对比 Python ~100MB）
- 支持 ~10000 并发 Agents

---

### DAG Operator (DAG 操作符)

**定义：** Nexa v0.9.7+ 引入的有向无环图拓扑操作符，用于表达复杂的多 Agent 协作模式。

**操作符列表：**

| 操作符 | 名称 | 说明 |
|-------|------|------|
| `>>` | 管道 | 顺序传递数据 |
| `|>>` | 分叉 (Fan-out) | 并行发送到多个 Agent |
| `&>>` | 合流 (Fan-in) | 合并多个结果 |
| `??` | 条件分支 | 根据条件选择路径 |
| `||` | 异步分叉 | 发送后不等待结果 |
| `&&` | 共识合流 | 需要所有 Agent 达成一致 |

**相关术语：** Flow, Pipeline, Orchestration

---

### Flow (流程)

**定义：** Nexa 语言的一等公民，用于编排 Agent 之间的协作流程和数据流转。

**相关术语：** DAG, Pipeline, Orchestration

**示例：**
```nexa
flow main {
    result = input >> AgentA >> AgentB;
}
```

---

### Intent Router (意图路由)

**定义：** Nexa 的 `match intent` 语法结构，基于语义相似度自动选择执行分支。

**相关术语：** Semantic Routing, Discovery Layer

**示例：**
```nexa
match user_input {
    intent("查询天气") => WeatherBot,
    intent("翻译文本") => Translator,
    _ => DefaultBot
}
```

---

### Nexa Language (Nexa 语言)

**定义：** 一门 Agent-Native 编程语言，专为智能体协作设计，具有五个一等公民：agent、tool、protocol、flow、test。

**相关术语：** AVM, Agent-Native, DSL

**核心特性：**
- 声明式 Agent 定义
- DAG 拓扑编排
- 语义级控制流
- 原生测试框架
- MCP 协议支持

---

### Semantic If (语义条件)

**定义：** Nexa 的 `semantic_if` 语法，基于语义相似度而非精确布尔值进行条件判断。

**相关术语：** Intent Router, Vector Similarity

**示例：**
```nexa
semantic_if input matches "紧急请求" {
    UrgentHandler.run(input);
}
```

---

### std (Standard Library)

**定义：** Nexa 内置标准库，提供文件系统、HTTP 请求、时间处理等常用能力。

**命名空间：**

| 命名空间 | 说明 |
|---------|------|
| `std.fs` | 文件系统操作 |
| `std.http` | HTTP 网络请求 |
| `std.time` | 时间日期操作 |
| `std.json` | JSON 数据处理 |
| `std.text` | 文本处理 |
| `std.hash` | 加密与编码 |
| `std.math` | 数学运算 |
| `std.regex` | 正则表达式 |
| `std.shell` | Shell 命令 |
| `std.ask_human` | 人机交互 |

**相关术语：** Tool, Capability

---

### WASM Sandbox (WASM 沙盒)

**定义：** AVM 中用于安全执行外部 Tool 的 WebAssembly 运行环境，提供资源限制和隔离。

**相关术语：** AVM, RBAC, Security

**资源限制：**
- 最大内存：16MB（256 页）
- 最大 CPU 时间：5 秒
- 最大文件大小：10MB
- 网络访问：默认禁用

---

### @budget Decorator (@预算修饰器)

**定义：** Nexa 的修饰器语法，用于控制 Agent 网络调用的经济约束。

**相关术语：** State Channel, Economy Layer

**示例：**
```nexa
@budget(max_tokens=1000, cost_limit=50)
agent PremiumService {
    prompt: "高质量服务"
}
```

---

### @role Decorator (@角色修饰器)

**定义：** Nexa 的 RBAC 修饰器，为 Agent 分配安全角色和权限级别。

**相关术语：** RBAC, Identity Layer, VC

**角色级别：**
- `admin` - 系统管理员
- `agent_standard` - 标准智能体
- `agent_readonly` - 只读智能体

---

### Agent Declaration (Agent 声明)

**定义：** Nexa 语言中定义智能体的语法结构，包含 role、prompt、model 等属性。

**相关术语：** DID Registration, Capability Schema

**示例：**
```nexa
agent Translator {
    role: "专业翻译",
    model: "deepseek/deepseek-chat",
    prompt: "翻译文本"
}
```

---

### Tool Declaration (Tool 声明)

**定义：** Nexa 语言中定义工具的语法结构，可被 Agent 调用。

**相关术语：** Capability, std, MCP

**示例：**
```nexa
tool calculator {
    description: "数学计算",
    input: expression: string,
    output: result: float
}
```

---

### Protocol Declaration (Protocol 声明)

**定义：** Nexa 语言中定义协议的语法结构，约束 Agent 之间的交互格式。

**相关术语：** RPC Interface, Transport Layer

**示例：**
```nexa
protocol AnalysisRequest {
    input: data: object,
    output: report: string
}
```

---

### Memory Engine (记忆引擎)

**定义：** Nexa 企业级特性，提供长期记忆、知识图谱和上下文压缩能力。

**相关术语：** L1/L2 Cache, Knowledge Graph

**组件：**
- 长期外接记忆系统
- 动态知识图谱映射
- 内置上下文压缩器

---

### L1/L2 Cache (多层缓存)

**定义：** Nexa 的语义计算缓存系统，L1 为内存热缓存，L2 为磁盘冷缓存。

**相关术语：** Semantic Match, Performance

**特性：**
- L1：极高频、极低延时请求拦截
- L2：持久化查询，TTL 和 LRU 驱逐
- 语义映射命中：相似意图也能命中缓存

---

## 缩写表

| 缩写 | 全称 | 中文 |
|------|------|------|
| **ANN** | Approximate Nearest Neighbor | 近似最近邻 |
| **AVM** | Agent Virtual Machine | 智能体虚拟机 |
| **DAG** | Directed Acyclic Graph | 有向无环图 |
| **DHT** | Distributed Hash Table | 分布式哈希表 |
| **DID** | Decentralized Identifier | 去中心化标识符 |
| **DSL** | Domain Specific Language | 领域特定语言 |
| **gRPC** | Google Remote Procedure Call | Google 远程过程调用 |
| **HNSW** | Hierarchical Navigable Small World | 分层可导航小世界 |
| **HITL** | Human-in-the-Loop | 人在回路 |
| **IVF** | Inverted File Index | 倒排文件索引 |
| **JSON** | JavaScript Object Notation | JavaScript 对象表示法 |
| **LRU** | Least Recently Used | 最近最少使用 |
| **MCP** | Model Context Protocol | 模型上下文协议 |
| **mTLS** | Mutual Transport Layer Security | 双向传输层安全 |
| **M2M** | Machine-to-Machine | 机器对机器 |
| **NAT** | Network Address Translation | 网络地址转换 |
| **P2P** | Peer-to-Peer | 对等网络 |
| **RBAC** | Role-Based Access Control | 基于角色的访问控制 |
| **RPC** | Remote Procedure Call | 远程过程调用 |
| **STUN** | Session Traversal Utilities for NAT | NAT 会话穿透工具 |
| **TLS** | Transport Layer Security | 传输层安全 |
| **TTL** | Time-To-Live | 生存时间 |
| **TURN** | Traversal Using Relays around NAT | 使用中继穿透 NAT |
| **VC** | Verifiable Credential | 可验证凭证 |
| **WASM** | WebAssembly | WebAssembly |

---

## 相关文档

- [README](./README.md) - 项目概述
- [ARCHITECTURE](./ARCHITECTURE.md) - 架构设计
- [NEXA_INTEGRATION](./NEXA_INTEGRATION.md) - Nexa 语言集成设计
- [开发者指南](./DEVELOPER_GUIDE.md) - 接入指南