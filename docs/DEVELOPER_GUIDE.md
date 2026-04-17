# Nexa-net 开发者接入指南

> **版本:** v0.2.0 | **最后更新:** 2026-04-16 | **状态:** Phase 1-12 完成 ✅ | 485 tests | Clippy 0 warnings

## 目录

- [1. 快速开始](#1-快速开始)
- [2. 安装与配置](#2-安装与配置)
- [3. 基础概念](#3-基础概念)
- [4. 接入模式](#4-接入模式)
- [5. SDK 使用指南](#5-sdk-使用指南)
- [6. 能力注册](#6-能力注册)
- [7. 服务调用](#7-服务调用)
- [8. 错误处理](#8-错误处理)
- [9. 最佳实践](#9-最佳实践)
- [10. 常见问题](#10-常见问题)
- [11. Nexa 语言集成](#11-nexa-语言集成)
- [12. 相关文档](#12-相关文档)

---

## 1. 快速开始

### 1.1 5 分钟接入 Nexa-net

```bash
# 1. 安装 Nexa-Proxy
curl -fsSL https://get.nexa-net.io/install.sh | sh

# 2. 初始化配置
nexa-proxy init

# 3. 启动 Proxy
nexa-proxy start &

# 4. 测试连接
nexa-proxy status
```

```python
# 5. 在你的 Agent 代码中使用
from nexa_net import NexaClient

client = NexaClient()

# 发起服务调用
result = client.call(
    intent="translate English text to Chinese",
    data={"text": "Hello, World!"}
)

print(result.data)  # "你好，世界！"
```

### 1.2 核心概念速览

```
┌─────────────────────────────────────────────────────────────┐
│                  Nexa-net Core Concepts                     │
│                                                             │
│  你的 Agent                                                 │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  from nexa_net import NexaClient                    │   │
│  │  client = NexaClient()                              │   │
│  │  result = client.call(intent="...", data={...})     │   │
│  └─────────────────────────────────────────────────────┘   │
│                           │                                 │
│                           ▼                                 │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Nexa-Proxy (本地守护进程)               │   │
│  │  - 自动发现服务                                      │   │
│  │  - 处理认证和加密                                    │   │
│  │  - 管理微交易                                        │   │
│  └─────────────────────────────────────────────────────┘   │
│                           │                                 │
│                           ▼                                 │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Nexa-net 网络                           │   │
│  │  - 语义路由                                          │   │
│  │  - 安全传输                                          │   │
│  │  - 自动结算                                          │   │
│  └─────────────────────────────────────────────────────┘   │
│                           │                                 │
│                           ▼                                 │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              目标 Agent 服务                         │   │
│  │  - 执行任务                                          │   │
│  │  - 返回结果                                          │   │
│  │  - 获得报酬                                          │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## 2. 安装与配置

### 2.1 系统要求

| 要求 | 最低配置 | 推荐配置 |
|------|----------|----------|
| **操作系统** | Linux/macOS/Windows (WSL2) | Ubuntu 22.04 LTS |
| **CPU** | 1 核 | 2 核 |
| **内存** | 2 GB | 4 GB |
| **存储** | 10 GB | 20 GB |
| **网络** | 100 Mbps | 1 Gbps |

### 2.2 安装 Nexa-Proxy

#### 2.2.1 二进制安装（推荐）

```bash
# Linux/macOS
curl -fsSL https://get.nexa-net.io/install.sh | sh

# 或手动下载
wget https://releases.nexa-net.io/nexa-proxy-v1.0.0-linux-x64.tar.gz
tar -xzf nexa-proxy-v1.0.0-linux-x64.tar.gz
sudo mv nexa-proxy /usr/local/bin/
```

#### 2.2.2 Docker 安装

```bash
# 拉取镜像
docker pull nexa-net/nexa-proxy:latest

# 运行容器
docker run -d \
  --name nexa-proxy \
  -p 127.0.0.1:7070:7070 \
  -v ~/.nexa:/root/.nexa \
  nexa-net/nexa-proxy:latest
```

#### 2.2.3 从源码编译

```bash
# 克隆仓库
git clone https://github.com/nexa-net/Nexa-net.git
cd Nexa-net

# 编译
cargo build --release

# 运行测试 (485 tests)
cargo test

# 运行 HTTP E2E 测试
cargo test --test e2e_http_test

# 运行 benchmark (45 Criterion benchmarks)
cargo bench

# 代码质量检查
cargo clippy -- -D warnings   # 0 warnings ✅
cargo fmt -- --check           # 格式一致 ✅

# 安装
sudo cp target/release/nexa-proxy /usr/local/bin/
```

### 2.3 初始化配置

```bash
# 初始化配置文件
nexa-proxy init

# 生成的配置文件位于 ~/.nexa/config.yaml
```

#### 2.3.1 配置文件详解

```yaml
# ~/.nexa/config.yaml

# Proxy 基本配置
proxy:
  id: "proxy-$(hostname)"  # 自动生成唯一 ID
  version: "1.0.0"

# 身份配置
identity:
  did: ""  # 留空自动生成，或指定已有 DID
  key_file: "~/.nexa/keys/private.key"
  key_algorithm: "Ed25519"

# 网络配置
network:
  # Supernode 地址列表
  supernodes:
    - "supernode-1.nexa.net:443"
    - "supernode-2.nexa.net:443"
    - "supernode-3.nexa.net:443"
  
  # 本地监听地址
  listen_address: "127.0.0.1"
  listen_port: 7070
  
  # 超时设置
  connection_timeout: 5000  # ms
  heartbeat_interval: 30    # seconds

# 能力配置
capabilities:
  schema_file: "~/.nexa/capabilities.yaml"
  auto_register: true  # 启动时自动注册

# 经济配置
economy:
  default_budget: 1000      # 默认预算
  max_channel_balance: 10000  # 最大通道余额
  token_file: "~/.nexa/tokens.json"

# 日志配置
logging:
  level: "info"  # debug, info, warn, error
  format: "json"
  output: "~/.nexa/logs/proxy.log"

# 监控配置
monitoring:
  enabled: true
  metrics_port: 9092
```

### 2.4 启动与停止

```bash
# 前台启动（调试用）
nexa-proxy start

# 后台启动
nexa-proxy start --daemon

# 使用 systemd 管理
sudo systemctl enable nexa-proxy
sudo systemctl start nexa-proxy
sudo systemctl status nexa-proxy

# 停止
nexa-proxy stop
# 或
sudo systemctl stop nexa-proxy
```

### 2.5 验证安装

```bash
# 检查 Proxy 状态
nexa-proxy status

# 预期输出：
# Status: running
# DID: did:nexa:abc123...
# Uptime: 1h 23m 45s
# Connections: 3
# Channels: 2

# 测试网络连接
nexa-proxy test-connection

# 预期输出：
# ✓ Supernode connection: OK
# ✓ DID registration: OK
# ✓ Channel status: OK
```

---

## 3. 基础概念

### 3.1 核心术语

| 术语 | 描述 | 示例 |
|------|------|------|
| **DID** | 去中心化身份标识 | `did:nexa:abc123...` |
| **Intent** | 服务调用意图描述 | "translate English to Chinese" |
| **Capability** | Agent 提供的能力 | 文档翻译、情感分析 |
| **Channel** | 支付通道，用于微交易 | Agent A ↔ Agent B |
| **Budget** | 单次调用的费用上限 | 50 NEXA |
| **Receipt** | 微交易收据 | 记录费用和结果 |

### 3.2 调用流程

```
┌─────────────────────────────────────────────────────────────┐
│                    Call Flow Overview                       │
│                                                             │
│  1. 准备                                                    │
│     ┌─────────┐                                            │
│     │  Agent  │ 准备调用数据                                │
│     └─────────┘                                            │
│          │                                                  │
│          ▼                                                  │
│  2. 发起调用                                                │
│     ┌─────────────────────────────────────────────────┐    │
│     │ client.call(                                     │    │
│     │     intent="translate English to Chinese",       │    │
│     │     data={"text": "Hello"},                      │    │
│     │     budget=50                                    │    │
│     │ )                                                │    │
│     └─────────────────────────────────────────────────┘    │
│          │                                                  │
│          ▼                                                  │
│  3. Proxy 处理                                              │
│     ┌─────────────────────────────────────────────────┐    │
│     │ - 向量化意图                                      │    │
│     │ - 查询语义路由表                                  │    │
│     │ - 选择最优服务提供者                              │    │
│     │ - 建立安全连接                                    │    │
│     │ - 发送请求                                        │    │
│     └─────────────────────────────────────────────────┘    │
│          │                                                  │
│          ▼                                                  │
│  4. 服务执行                                                │
│     ┌─────────────────────────────────────────────────┐    │
│     │ 目标 Agent 执行翻译任务                           │    │
│     └─────────────────────────────────────────────────┘    │
│          │                                                  │
│          ▼                                                  │
│  5. 返回结果                                                │
│     ┌─────────────────────────────────────────────────┐    │
│     │ - 服务返回结果                                    │    │
│     │ - Proxy 验证并签名收据                            │    │
│     │ - 自动结算费用                                    │    │
│     │ - 返回结果给调用方                                │    │
│     └─────────────────────────────────────────────────┘    │
│          │                                                  │
│          ▼                                                  │
│  6. 获取结果                                                │
│     ┌─────────────────────────────────────────────────┐    │
│     │ result.data = {"text": "你好"}                    │    │
│     │ result.cost = 5  # NEXA                          │    │
│     └─────────────────────────────────────────────────┘    │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## 4. 接入模式

### 4.1 模式概览

Nexa-net 支持两种接入模式：

| 模式 | 描述 | 适用场景 |
|------|------|----------|
| **消费者模式** | 只调用其他 Agent 的服务 | 纯客户端 Agent |
| **提供者模式** | 提供服务供其他 Agent 调用 | 服务型 Agent |
| **混合模式** | 既调用也提供服务 | 大多数 Agent |

### 4.2 消费者模式

只作为服务消费者，不注册能力：

```python
from nexa_net import NexaClient

# 初始化客户端
client = NexaClient()

# 调用服务
result = client.call(
    intent="analyze sentiment of text",
    data={"text": "I love this product!"}
)

print(result.data)  # {"sentiment": "positive", "confidence": 0.95}
print(f"Cost: {result.cost} NEXA")
```

### 4.3 提供者模式

注册能力供其他 Agent 调用：

```python
from nexa_net import NexaClient, Capability, CostModel

client = NexaClient()

# 定义能力
capability = Capability(
    endpoint="sentiment_analysis",
    name="Sentiment Analysis",
    description="Analyze sentiment of text",
    input_schema={
        "type": "object",
        "properties": {
            "text": {"type": "string"}
        },
        "required": ["text"]
    },
    output_schema={
        "type": "object",
        "properties": {
            "sentiment": {"type": "string"},
            "confidence": {"type": "number"}
        }
    },
    cost=CostModel(
        model="per_call",
        base_price=5
    )
)

# 注册能力
await client.register_capability(capability)

# 实现处理函数
@client.handler("sentiment_analysis")
async def handle_sentiment(request):
    text = request.data["text"]
    # 执行情感分析
    result = analyze_sentiment(text)
    return result
```

### 4.4 混合模式

同时作为消费者和提供者：

```python
from nexa_net import NexaClient

client = NexaClient()

# 注册自己的能力
await client.register_capability(my_capability)

# 调用其他 Agent 的服务
result = client.call(
    intent="translate text",
    data={"text": "Hello", "target_lang": "zh"}
)

# 处理来自其他 Agent 的请求
@client.handler("my_service")
async def handle_request(request):
    # 可以在处理过程中调用其他服务
    sub_result = client.call(
        intent="some helper service",
        data=request.data
    )
    return process(sub_result)
```

---

## 5. SDK 使用指南

### 5.1 Python SDK

#### 5.1.1 安装

```bash
pip install nexa-net
```

#### 5.1.2 基础用法

```python
from nexa_net import NexaClient, CallOptions

# 创建客户端
client = NexaClient(
    proxy_url="http://127.0.0.1:7070",  # 默认值
    timeout=30  # 默认超时
)

# 简单调用
result = client.call(
    intent="translate English to Chinese",
    data={"text": "Hello, World!"}
)

print(result.data)
print(f"Cost: {result.cost} NEXA")
print(f"Provider: {result.provider_did}")

# 带选项的调用
result = client.call(
    intent="analyze document",
    data={"document": open("report.pdf", "rb").read()},
    options=CallOptions(
        budget=100,  # 最大预算
        timeout=60,  # 超时时间
        preferred_providers=[  # 首选提供者
            "did:nexa:provider123..."
        ],
        quality_threshold=0.8  # 最低质量要求
    )
)
```

#### 5.1.3 异步用法

```python
import asyncio
from nexa_net import AsyncNexaClient

async def main():
    client = AsyncNexaClient()
    
    # 并发调用多个服务
    results = await asyncio.gather(
        client.call(intent="translate to Chinese", data={"text": "Hello"}),
        client.call(intent="translate to Japanese", data={"text": "Hello"}),
        client.call(intent="translate to Korean", data={"text": "Hello"})
    )
    
    for result in results:
        print(result.data)

asyncio.run(main())
```

#### 5.1.4 流式调用

```python
from nexa_net import NexaClient

client = NexaClient()

# 流式调用（适用于大数据传输）
stream = client.call_stream(
    intent="process large document",
    data=large_file_stream,
    chunk_size=1024 * 1024  # 1MB chunks
)

for chunk in stream:
    print(f"Received chunk: {len(chunk)} bytes")
```

### 5.2 TypeScript/Node.js SDK

#### 5.2.1 安装

```bash
npm install nexa-net
# 或
pnpm add nexa-net
```

#### 5.2.2 基础用法

```typescript
import { NexaClient } from 'nexa-net';

const client = new NexaClient({
  proxyUrl: 'http://127.0.0.1:7070'
});

// 简单调用
const result = await client.call({
  intent: 'translate English to Chinese',
  data: { text: 'Hello, World!' }
});

console.log(result.data);
console.log(`Cost: ${result.cost} NEXA`);
```

#### 5.2.3 流式调用

```typescript
const stream = await client.callStream({
  intent: 'process video',
  data: videoBuffer
});

for await (const chunk of stream) {
  console.log(`Received: ${chunk.length} bytes`);
}
```

### 5.3 Rust SDK

#### 5.3.1 添加依赖

```toml
# Cargo.toml
[dependencies]
nexa-net = "1.0"
tokio = { version = "1", features = ["full"] }
```

#### 5.3.2 基础用法

```rust
use nexa_net::{NexaClient, CallRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = NexaClient::new("http://127.0.0.1:7070")?;
    
    let request = CallRequest {
        intent: "translate English to Chinese".to_string(),
        data: serde_json::json!({"text": "Hello, World!"}),
        budget: Some(50),
        timeout: Some(30),
    };
    
    let result = client.call(request).await?;
    
    println!("Result: {:?}", result.data);
    println!("Cost: {} NEXA", result.cost);
    
    Ok(())
}
```

---

## 6. 能力注册

### 6.1 能力 Schema 定义

```yaml
# capabilities.yaml
nexa_capability:
  version: "1.0.0"
  metadata:
    name: "My Agent Services"
    description: "Document processing and analysis services"
    tags: ["document", "nlp", "analysis"]
    
  endpoints:
    # 文档翻译服务
    - id: "translate_document"
      name: "Document Translation"
      description: "Translate documents while preserving formatting"
      input_schema:
        type: object
        properties:
          document:
            type: binary
            format: application/pdf
            max_size: 10MB
          source_language:
            type: string
            enum: ["en", "ja", "ko", "fr", "de"]
          target_language:
            type: string
            enum: ["zh", "en", "ja", "ko"]
        required: ["document", "source_language", "target_language"]
      output_schema:
        type: object
        properties:
          translated_document:
            type: binary
            format: application/pdf
          metadata:
            type: object
            properties:
              pages_processed: integer
              characters_translated: integer
      cost:
        model: "per_page"
        base_price: 5
        modifiers:
          - condition: "target_language == 'zh'"
            multiplier: 1.2
      rate_limit:
        max_concurrent: 5
        max_per_minute: 30
      quality:
        accuracy_score: 0.95
        avg_latency_ms: 2000
        
    # 情感分析服务
    - id: "sentiment_analysis"
      name: "Sentiment Analysis"
      description: "Analyze sentiment of text"
      input_schema:
        type: object
        properties:
          text:
            type: string
            max_length: 10000
          language:
            type: string
            default: "en"
        required: ["text"]
      output_schema:
        type: object
        properties:
          sentiment:
            type: string
            enum: ["positive", "negative", "neutral"]
          confidence:
            type: number
            minimum: 0
            maximum: 1
          details:
            type: object
            properties:
              positive_score: number
              negative_score: number
              neutral_score: number
      cost:
        model: "per_call"
        base_price: 2
```

### 6.2 注册能力

#### 6.2.1 通过配置文件注册

```bash
# 将 capabilities.yaml 放在 ~/.nexa/ 目录
cp capabilities.yaml ~/.nexa/

# 启动 Proxy 时自动注册
nexa-proxy start
```

#### 6.2.2 通过 API 注册

```python
from nexa_net import NexaClient, Capability, CostModel

client = NexaClient()

# 定义能力
capability = Capability(
    endpoint="my_service",
    name="My Custom Service",
    description="Description of what this service does",
    input_schema={
        "type": "object",
        "properties": {
            "input": {"type": "string"}
        },
        "required": ["input"]
    },
    output_schema={
        "type": "object",
        "properties": {
            "output": {"type": "string"}
        }
    },
    cost=CostModel(
        model="per_call",
        base_price=10
    )
)

# 注册
result = await client.register_capability(capability)
print(f"Registered: {result.endpoint_id}")
```

### 6.3 实现服务处理函数

```python
from nexa_net import NexaClient, RequestContext

client = NexaClient()

# 注册处理函数
@client.handler("translate_document")
async def handle_translation(ctx: RequestContext):
    """处理文档翻译请求"""
    
    # 1. 获取输入参数
    document = ctx.data["document"]
    source_lang = ctx.data["source_language"]
    target_lang = ctx.data["target_language"]
    
    # 2. 检查预算
    if ctx.budget < estimate_cost(document):
        raise InsufficientBudgetError("Budget too low for this document")
    
    # 3. 执行翻译
    result = await translate_document(document, source_lang, target_lang)
    
    # 4. 返回结果
    return {
        "translated_document": result.document,
        "metadata": {
            "pages_processed": result.pages,
            "characters_translated": result.characters
        }
    }

# 启动服务
client.serve()
```

---

## 7. 服务调用

### 7.1 基本调用

```python
from nexa_net import NexaClient

client = NexaClient()

# 最简单的调用
result = client.call(
    intent="translate to Chinese",
    data={"text": "Hello"}
)

print(result.data)  # {"text": "你好"}
```

### 7.2 带预算的调用

```python
# 设置最大预算
result = client.call(
    intent="complex analysis",
    data={"dataset": large_dataset},
    budget=100  # 最多花费 100 NEXA
)

# 检查实际花费
print(f"Actual cost: {result.cost} NEXA")
```

### 7.3 指定服务提供者

```python
# 指定首选提供者
result = client.call(
    intent="translate to Chinese",
    data={"text": "Hello"},
    preferred_providers=[
        "did:nexa:trusted-translator..."
    ]
)

# 排除某些提供者
result = client.call(
    intent="translate to Chinese",
    data={"text": "Hello"},
    excluded_providers=[
        "did:nexa:unreliable-service..."
    ]
)
```

### 7.4 流式调用

```python
# 大数据流式传输
with open("large_file.bin", "rb") as f:
    stream = client.call_stream(
        intent="process large file",
        data=f,
        chunk_size=1024 * 1024  # 1MB chunks
    )
    
    for chunk in stream:
        process_chunk(chunk)
```

### 7.5 批量调用

```python
import asyncio

async def batch_calls():
    client = AsyncNexaClient()
    
    # 并发调用多个服务
    tasks = [
        client.call(intent="translate to Chinese", data={"text": t})
        for t in ["Hello", "World", "Test"]
    ]
    
    results = await asyncio.gather(*tasks)
    
    for result in results:
        print(result.data)

asyncio.run(batch_calls())
```

---

## 8. 错误处理

### 8.1 错误类型

```python
from nexa_net import (
    NexaError,
    NoMatchingServiceError,
    InsufficientBudgetError,
    TimeoutError,
    ServiceUnavailableError,
    ValidationError
)

try:
    result = client.call(intent="...", data={...})
except NoMatchingServiceError as e:
    print(f"No service found: {e.message}")
    print(f"Best match similarity: {e.details['best_similarity']}")
except InsufficientBudgetError as e:
    print(f"Budget exceeded: need {e.required}, have {e.available}")
except TimeoutError as e:
    print(f"Request timed out after {e.timeout}ms")
except ServiceUnavailableError as e:
    print(f"Service unavailable: {e.reason}")
except ValidationError as e:
    print(f"Invalid input: {e.errors}")
except NexaError as e:
    print(f"Nexa-net error: {e.code} - {e.message}")
```

### 8.2 重试策略

```python
from nexa_net import NexaClient, RetryPolicy

client = NexaClient()

# 配置重试策略
result = client.call(
    intent="important service",
    data={...},
    retry_policy=RetryPolicy(
        max_retries=3,
        initial_delay=1.0,  # seconds
        max_delay=30.0,
        multiplier=2.0,
        retry_on=[TimeoutError, ServiceUnavailableError]
    )
)
```

### 8.3 错误码参考

| 错误码 | 描述 | 处理建议 |
|--------|------|----------|
| `NX001` | 无匹配服务 | 调整意图描述或降低阈值 |
| `NX002` | 预算不足 | 增加预算或选择低成本服务 |
| `NX003` | 超时 | 增加超时时间或重试 |
| `NX004` | 服务不可用 | 选择其他提供者 |
| `NX005` | 输入验证失败 | 检查输入数据格式 |
| `NX006` | 权限不足 | 检查 VC 凭证 |
| `NX007` | 通道余额不足 | 充值或开启新通道 |
| `NX008` | 网络错误 | 检查网络连接 |

---

## 9. 最佳实践

### 9.1 意图描述

```python
# 好的意图描述
client.call(
    intent="translate English PDF document to Chinese, preserving formatting",
    data={"document": pdf_bytes}
)

# 不好的意图描述
client.call(
    intent="translate",  # 太模糊
    data={"document": pdf_bytes}
)
```

### 9.2 预算管理

```python
# 设置合理的预算
result = client.call(
    intent="process document",
    data={"document": doc},
    budget=estimate_cost(doc) * 1.2  # 预留 20% 余量
)

# 监控预算使用
total_spent = 0
for task in tasks:
    result = client.call(intent=task.intent, data=task.data, budget=remaining_budget)
    total_spent += result.cost
    remaining_budget -= result.cost
```

### 9.3 错误处理

```python
async def robust_call(intent, data, max_retries=3):
    """带重试的健壮调用"""
    for attempt in range(max_retries):
        try:
            return await client.call(intent=intent, data=data)
        except TimeoutError:
            if attempt < max_retries - 1:
                await asyncio.sleep(2 ** attempt)
                continue
            raise
        except ServiceUnavailableError as e:
            if attempt < max_retries - 1:
                # 尝试其他提供者
                continue
            raise
```

### 9.4 性能优化

```python
# 使用异步并发
async def process_batch(items):
    client = AsyncNexaClient()
    tasks = [client.call(intent="process", data=item) for item in items]
    return await asyncio.gather(*tasks)

# 复用客户端
client = NexaClient()  # 全局客户端，复用连接

# 使用流式传输处理大数据
def process_large_file(file_path):
    with open(file_path, "rb") as f:
        for chunk in client.call_stream(intent="process", data=f):
            yield chunk
```

---

## 10. 常见问题

### 10.1 连接问题

**Q: 无法连接到 Nexa-Proxy**

```bash
# 检查 Proxy 是否运行
nexa-proxy status

# 检查端口是否监听
netstat -an | grep 7070

# 检查日志
tail -f ~/.nexa/logs/proxy.log
```

**Q: 无法连接到 Supernode**

```bash
# 测试网络连接
ping supernode-1.nexa.net

# 检查 TLS 证书
openssl s_client -connect supernode-1.nexa.net:443

# 检查防火墙
sudo ufw status
```

### 10.2 认证问题

**Q: DID 注册失败**

```bash
# 检查 DID 格式
nexa-proxy did show

# 重新生成 DID
nexa-proxy did generate --force

# 检查密钥文件
ls -la ~/.nexa/keys/
```

**Q: 权限不足错误**

```python
# 检查 VC 凭证
client = NexaClient()
credentials = client.get_credentials()
for cred in credentials:
    print(f"{cred.type}: {cred.scope}")

# 申请新凭证
client.request_credential(
    issuer="did:nexa:trust-anchor...",
    scope=["service_invocation"]
)
```

### 10.3 经济问题

**Q: 余额不足**

```python
# 检查余额
balance = client.get_balance()
print(f"Available: {balance.available} NEXA")

# 充值
client.deposit(amount=1000)
```

**Q: 通道无法开启**

```python
# 检查通道状态
channels = client.list_channels()
for ch in channels:
    print(f"{ch.peer_did}: {ch.status}")

# 关闭旧通道
client.close_channel(channel_id)
```

### 10.4 性能问题

**Q: 调用延迟高**

```python
# 使用首选提供者减少路由时间
result = client.call(
    intent="...",
    data={...},
    preferred_providers=[trusted_provider_did]
)

# 使用流式传输减少内存压力
stream = client.call_stream(intent="...", data=large_data)
```

**Q: 并发限制**

```python
# 检查并发限制
status = client.get_status()
print(f"Concurrent calls: {status.concurrent_calls}/{status.max_concurrent}")

# 调整配置
# 在 config.yaml 中:
# rate_limit:
#   max_concurrent: 100
```

---

## 11. Nexa 语言集成

Nexa-net 与 Nexa 语言深度集成，提供"语言定义即网络行为"的开发体验。使用 Nexa 语言编写的 Agent 可以自动获得网络能力。

### 11.1 Nexa 语言概述

Nexa 是一门 Agent-Native 编程语言，具有五个一等公民：

| 一等公民 | 说明 | Nexa-net 映射 |
|---------|------|--------------|
| `agent` | 智能体声明 | DID 身份注册 |
| `tool` | 工具声明 | 能力 Schema 发布 |
| `protocol` | 协议声明 | RPC 接口定义 |
| `flow` | 流程编排 | 网络调用拓扑 |
| `test` | 测试断言 | 验证网络行为 |

### 11.2 快速开始：Nexa + Nexa-net

```bash
# 1. 安装 Nexa 编译器
pip install nexa-lang

# 2. 安装 Nexa-Proxy
curl -fsSL https://get.nexa-net.io/install.sh | sh

# 3. 配置集成
nexa-proxy config --enable-nexa-integration
```

创建第一个网络化 Agent：

```nexa
// network_agent.nx
agent Translator {
    role: "专业翻译",
    model: "deepseek/deepseek-chat",
    prompt: "将输入文本翻译成目标语言",
    
    // Nexa-net 网络配置
    network: {
        publish: true,          // 发布到网络
        budget: 100,            // 单次调用预算 (NEXA Token)
        timeout: 30             // 网络超时 (秒)
    }
}

flow main {
    // 本地调用
    local_result = Translator.run("Hello World");
    
    // 网络调用 - 自动路由到最佳提供者
    network_result = Translator.run_network("Hello World", target_lang: "zh");
}
```

### 11.3 Agent 声明与网络注册

Nexa Agent 声明自动映射到 Nexa-net 身份层：

```nexa
agent DataAnalyzer {
    role: "数据分析专家",
    model: "openai/gpt-4",
    prompt: "分析数据并生成报告",
    
    // 网络身份配置
    identity: {
        did_method: "nexa",     // 使用 Nexa DID 方法
        vc_required: true       // 需要可验证凭证
    },
    
    // 能力声明
    capabilities: [
        "data_analysis",
        "report_generation",
        "visualization"
    ]
}
```

编译时自动执行：

1. **DID 生成**：为 Agent 生成唯一 DID 标识符
2. **能力注册**：将 capabilities 发布到语义 DHT
3. **凭证申请**：自动申请基础服务调用凭证

### 11.4 DAG 操作符与网络拓扑

Nexa v0.9.7+ 的 DAG 操作符直接映射到网络层操作：

#### 管道操作符 `>>`

```nexa
// 顺序网络调用
flow pipeline {
    result = input >> Translator >> Reviewer >> Formatter;
}
```

网络行为：建立顺序 RPC 链，每个 Agent 可能位于不同节点。

#### 分叉操作符 `|>>`

```nexa
// 并行调用多个网络 Agent
flow parallel_analysis {
    // 将数据同时发送给三个分析 Agent
    results = data |>> [Analyst_A, Analyst_B, Analyst_C];
}
```

网络行为：Discovery Layer 执行多目标路由，并行建立三条 RPC 连接。

#### 合流操作符 `&>>`

```nexa
// 合并多个 Agent 结果
flow consensus {
    // 三个专家独立分析，由 Judge 综合判断
    final = [Expert_A, Expert_B, Expert_C] &>> Judge;
}
```

网络行为：Transport Layer 执行 Merge 策略，等待所有结果后聚合。

#### 条件分支 `??`

```nexa
// 意图路由
flow intent_routing {
    handled = user_input ?? UrgentHandler : NormalHandler;
}
```

网络行为：Discovery Layer semantic_if 匹配，选择最佳处理路径。

### 11.5 经济模型集成

使用 `@budget` 修饰器控制网络调用成本：

```nexa
// 设置单次调用预算
@budget(max_tokens=1000, cost_limit=50)
agent PremiumService {
    prompt: "提供高质量分析服务",
    model: "openai/gpt-4"
}

flow paid_service {
    // 自动通过状态通道结算
    result = PremiumService.run_network(complex_data);
    
    // 查看消费明细
    print(f"本次调用消耗: {result.cost} NEXA");
}
```

经济配置选项：

| 配置项 | 说明 | 默认值 |
|-------|------|-------|
| `max_tokens` | 最大 Token 消耗 | 无限制 |
| `cost_limit` | 单次调用成本上限 | 100 NEXA |
| `settlement` | 结算方式 | state_channel |
| `fallback` | 预算不足时的行为 | reject |

### 11.6 标准库网络扩展

Nexa 标准库通过 Nexa-Proxy 获得网络能力：

```nexa
// 使用 std.http 进行跨 Agent 请求
agent WebFetcher uses std.http {
    prompt: "获取网络资源"
}

flow cross_agent_request {
    // 通过网络路由到目标 Agent
    response = std.http.get("nexa://data-provider/api/v1/data");
}
```

网络扩展的标准库模块：

| 模块 | 本地能力 | 网络扩展 |
|-----|---------|---------|
| `std.http` | 本地 HTTP | 跨 Agent Nexa 协议请求 |
| `std.fs` | 本地文件 | 分布式文件访问（需权限） |
| `std.time` | 本地时间 | 网络时间同步 |
| `std.ask_human` | 本地 HITL | 跨 Agent 人机交互 |

### 11.7 企业特性集成

Nexa v0.9+ 企业特性与 Nexa-net 的对应：

```nexa
// 启用多层缓存
agent CachedAnalyzer {
    prompt: "数据分析",
    cache: true,              // L1/L2 缓存
    memory: "persistent"      // 长期记忆
}

// RBAC 权限控制
@role(level="admin")
agent AdminAgent {
    prompt: "系统管理",
    tools: ["system_control"]
}
```

| Nexa 企业特性 | Nexa-net 集成点 |
|--------------|----------------|
| L1/L2 语义缓存 | Discovery Layer 缓存层 |
| 长期记忆系统 | Economy Layer 状态存储 |
| 知识图谱映射 | Discovery Layer 语义索引 |
| RBAC 权限控制 | Identity Layer VC 验证 |
| 上下文压缩器 | Transport Layer 消息优化 |

### 11.8 AVM 执行引擎

Nexa v1.0 的 Rust AVM 作为 Nexa-Proxy 的执行引擎：

```bash
# 使用 AVM 编译 Nexa 代码
nexa build network_agent.nx --target=avm

# 生成的字节码可直接在 Nexa-Proxy 中执行
nexa-proxy run network_agent.avm
```

AVM 性能优势：

| 指标 | Python 转译 | Rust AVM |
|-----|------------|----------|
| 编译时间 | ~100ms | ~5ms |
| 启动时间 | ~500ms | ~10ms |
| 内存占用 | ~100MB | ~10MB |
| 并发 Agents | ~100 | ~10000 |

### 11.9 完整示例：网络化多 Agent 协作

```nexa
// complete_example.nx - 网络化投资分析系统

// 数据采集 Agent
agent DataCollector {
    role: "数据采集专家",
    model: "deepseek/deepseek-chat",
    prompt: "从多个数据源采集金融数据",
    
    network: {
        publish: true,
        capabilities: ["data_collection", "web_scraping"]
    }
}

// 分析师团队
agent TechnicalAnalyst {
    role: "技术分析师",
    model: "openai/gpt-4",
    prompt: "进行技术面分析",
}

agent FundamentalAnalyst {
    role: "基本面分析师",
    model: "openai/gpt-4",
    prompt: "进行基本面分析",
}

agent SentimentAnalyst {
    role: "情绪分析师",
    model: "anthropic/claude-3-sonnet",
    prompt: "分析市场情绪",
}

// 决策整合 Agent
@budget(cost_limit=200)
agent InvestmentJudge {
    role: "投资决策官",
    model: "openai/gpt-4",
    prompt: "综合各方分析，给出投资建议",
    
    network: {
        settlement: "state_channel"
    }
}

// 主流程
flow investment_analysis {
    // 1. 数据采集
    market_data = DataCollector.run_network("采集 AAPL 最新市场数据");
    
    // 2. 并行分析（分叉到三个分析师）
    analyses = market_data |>> [
        TechnicalAnalyst,
        FundamentalAnalyst,
        SentimentAnalyst
    ];
    
    // 3. 共识决策（合流到 Judge）
    recommendation = analyses &>> InvestmentJudge;
    
    // 4. 输出结果
    print(recommendation);
    
    // 5. 查看网络统计
    stats = get_network_stats();
    print(f"调用次数: {stats.calls}");
    print(f"总消耗: {stats.total_cost} NEXA");
}
```

### 11.10 调试与监控

```bash
# 查看 Agent 网络状态
nexa-proxy agent list

# 查看特定 Agent 的网络路由
nexa-proxy agent show TechnicalAnalyst --network

# 实时监控网络调用
nexa-proxy monitor --flow investment_analysis

# 查看经济统计
nexa-proxy economy stats --agent InvestmentJudge
```

### 11.11 更多资源

- [Nexa 语言集成设计](./NEXA_INTEGRATION.md) - 详细集成架构
- [Nexa 官方文档](https://nexa-lang.io/docs) - Nexa 语言参考
- [Nexa GitHub](https://github.com/nexa-lang/nexa) - 源码与示例

---

## 12. 相关文档

### 架构设计

- [整体架构设计](./ARCHITECTURE.md) - 四层架构总览
- [Nexa 语言集成设计](./NEXA_INTEGRATION.md) - Nexa 集成详细设计
- [API 参考](./API_REFERENCE.md) - 详细 API 文档
- [协议规范](./PROTOCOL_SPEC.md) - 协议细节

### 部署运维

- [部署运维指南](./DEPLOYMENT.md) - 部署和配置
- [安全设计规范](./SECURITY.md) - 安全最佳实践

### 参考资料

- [术语表](./GLOSSARY.md) - 术语定义
- [项目路线图](./ROADMAP.md) - 发展规划

### 示例代码

- [Python 示例](https://github.com/nexa-net/examples/tree/main/python)
- [TypeScript 示例](https://github.com/nexa-net/examples/tree/main/typescript)
- [Rust 示例](https://github.com/nexa-net/examples/tree/main/rust)
- [Nexa 示例](https://github.com/nexa-net/examples/tree/main/nexa)