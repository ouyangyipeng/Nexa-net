# Nexa 语言与 Nexa-net 集成设计

> **版本:** v1.0.0-draft | **最后更新:** 2026-03-30

## 目录

- [1. 概述](#1-概述)
- [2. Nexa 语言简介](#2-nexa-语言简介)
- [3. 集成架构](#3-集成架构)
- [4. Agent 声明与网络注册](#4-agent-声明与网络注册)
- [5. DAG 操作符与网络拓扑](#5-dag-操作符与网络拓扑)
- [6. 语义路由集成](#6-语义路由集成)
- [7. AVM 作为执行引擎](#7-avm-作为执行引擎)
- [8. 经济模型集成](#8-经济模型集成)
- [9. 安全沙盒集成](#9-安全沙盒集成)
- [10. 开发者体验](#10-开发者体验)
- [11. 相关文档](#11-相关文档)

---

## 1. 概述

### 1.1 设计目标

**Nexa** 是一门为 LLM 和智能体系统量身定制的 **Agent-Native 编程语言**。**Nexa-net** 是一个去中心化的 Agent 通讯基础设施。两者的集成将实现：

```
┌─────────────────────────────────────────────────────────────┐
│                    Integration Vision                       │
│                                                             │
│  Nexa Language          Nexa-net Network                    │
│  ┌─────────────┐        ┌─────────────────────────────┐    │
│  │ Agent 声明  │ ──────▶│ 网络节点注册                 │    │
│  │ Tool 定义   │ ──────▶│ 能力 Schema 发布            │    │
│  │ Protocol    │ ──────▶│ 输入输出约束                │    │
│  │ DAG 操作符  │ ──────▶│ 网络拓扑编排                │    │
│  │ 语义路由    │ ──────▶│ DHT 语义查询                │    │
│  │ AVM 执行    │ ──────▶│ Proxy 执行引擎              │    │
│  └─────────────┘        └─────────────────────────────┘    │
│                                                             │
│  目标：用 Nexa 语法编写 Agent，自动接入 Nexa-net 网络        │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### 1.2 核心价值

| 维度 | 传统方式 | Nexa + Nexa-net |
|------|----------|-----------------|
| **Agent 定义** | 手写 Python/TypeScript | 声明式 Nexa 语法 |
| **网络接入** | 手动配置 API 端点 | 自动注册到网络 |
| **服务发现** | 硬编码 URL | 语义路由自动匹配 |
| **数据格式** | 手写 JSON Schema | Protocol 声明自动生成 |
| **并发编排** | 手写异步代码 | DAG 操作符原生支持 |
| **安全隔离** | 依赖外部容器 | WASM 沙盒内置 |

---

## 2. Nexa 语言简介

### 2.1 核心一等公民

Nexa 语言定义了五个核心一等公民：

```nexa
// 1. tool - 工具声明
tool Calculator {
    description: "Perform basic math operations",
    parameters: {"expression": "string"}
}

// 2. protocol - 协议声明（输出约束）
protocol AnalysisReport {
    title: "string",
    sentiment: "string",
    confidence: "number"
}

// 3. agent - 智能体声明
@limit(max_tokens=2048)
agent FinancialAnalyst implements AnalysisReport uses Calculator {
    role: "Senior Financial Advisor",
    model: "claude-3.5-sonnet",
    prompt: "Analyze financial data and output standard reports."
}

// 4. flow - 流程编排
flow main {
    raw_data = SearchTool.run("AAPL Q3 index");
    summary = raw_data >> FinancialAnalyst >> Formatter;
    print(summary);
}

// 5. test - 测试声明
test "financial_analysis_basic" {
    result = FinancialAnalyst.run("Tesla revenue 2023");
    assert "包含具体的财务分析" against result;
}
```

### 2.2 DAG 操作符

Nexa v0.9.7+ 引入了强大的 DAG 操作符：

```nexa
// 分叉：并行发送到多个 Agent
results = input |>> [Researcher, Analyst, Writer];

// 合流：合并多个结果
report = [Researcher, Analyst] &>> Reviewer;

// 条件分支：根据输入选择路径
result = input ?? UrgentHandler : NormalHandler;

// Fire-forget：不等待结果
input || [Logger, Analytics];

// 共识合流：需要 Agent 达成一致
consensus = [Agent1, Agent2] && JudgeAgent;
```

### 2.3 语义路由

```nexa
// 意图路由
match user_req {
    intent("查询天气") => WeatherBot.run(user_req),
    intent("查询股市") => StockBot.run(user_req),
    _ => SmallTalkBot.run(user_req)
}

// 语义条件分支
semantic_if "包含具体的日期和地点" fast_match r"\d{4}-\d{2}-\d{2}" against user_input {
    schedule_tool.run(user_input);
} else {
    print("需要进一步澄清");
}
```

### 2.4 AVM (Agent Virtual Machine)

Nexa v1.0 引入了基于 Rust 的高性能 AVM：

| 特性 | Python 转译器 | Rust AVM |
|------|--------------|----------|
| **编译时间** | ~100ms | ~5ms |
| **启动时间** | ~500ms | ~10ms |
| **内存占用** | ~100MB | ~10MB |
| **并发 Agents** | ~100 | ~10000 |

---

## 3. 集成架构

### 3.1 整体架构

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    Nexa + Nexa-net Integration Architecture            │
│                                                                         │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                      Developer Layer                             │   │
│  │  ┌─────────────────────────────────────────────────────────┐    │   │
│  │  │                    Nexa Source Code                      │    │   │
│  │  │  agent MyAgent { ... }                                   │    │   │
│  │  │  tool MyTool { ... }                                     │    │   │
│  │  │  flow main { ... }                                       │    │   │
│  │  └─────────────────────────────────────────────────────────┘    │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                   │                                     │
│                                   ▼                                     │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                      Compilation Layer                          │   │
│  │  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐  │   │
│  │  │ Nexa Compiler   │  │ Schema Generator│  │ Network Config  │  │   │
│  │  │ (Lexer/Parser)  │  │ (Protocol→JSON) │  │ Generator       │  │   │
│  │  └─────────────────┘  └─────────────────┘  └─────────────────┘  │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                   │                                     │
│                                   ▼                                     │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                      Runtime Layer                              │   │
│  │  ┌─────────────────────────────────────────────────────────┐    │   │
│  │  │                    Nexa-Proxy + AVM                      │    │   │
│  │  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐      │    │   │
│  │  │  │ AVM Runtime │  │ WASM Sandbox│  │ Network API │      │    │   │
│  │  │  └─────────────┘  └─────────────┘  └─────────────┘      │    │   │
│  │  └─────────────────────────────────────────────────────────┘    │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                   │                                     │
│                                   ▼                                     │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                      Network Layer                              │   │
│  │  ┌─────────────────────────────────────────────────────────┐    │   │
│  │  │                    Nexa-net Network                      │    │   │
│  │  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐      │    │   │
│  │  │  │ Supernode   │  │ Semantic DHT│  │ State Channel│     │    │   │
│  │  │  └─────────────┘  └─────────────┘  └─────────────┘      │    │   │
│  │  └─────────────────────────────────────────────────────────┘    │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

### 3.2 编译流程

```
┌─────────────────────────────────────────────────────────────┐
│                    Compilation Pipeline                      │
│                                                             │
│  Nexa Source                                                │
│  ┌─────────────────────────────────────────────────────┐   │
│  │ agent Translator {                                  │   │
│  │     model: "gpt-4",                                │   │
│  │     prompt: "Translate text"                        │   │
│  │ }                                                   │   │
│  │                                                     │   │
│  │ tool NexaCall {                                     │   │
│  │     nexa_net: true,                                 │   │
│  │     intent: "translate text"                        │   │
│  │ }                                                   │   │
│  │                                                     │   │
│  │ flow main {                                         │   │
│  │     result = input |>> [Translator, Reviewer];      │   │
│  │ }                                                   │   │
│  └─────────────────────────────────────────────────────┘   │
│                           │                                 │
│                           ▼                                 │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Nexa Compiler (Rust AVM)               │   │
│  │  1. Lexer → Tokens                                  │   │
│  │  2. Parser → AST                                    │   │
│  │  3. Type Checker → Validated AST                    │   │
│  │  4. Code Generator → Bytecode + Config              │   │
│  └─────────────────────────────────────────────────────┘   │
│                           │                                 │
│                           ▼                                 │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Generated Artifacts                     │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  │   │
│  │  │ AVM Bytecode│  │ Capability  │  │ Network     │  │   │
│  │  │ (.nxbc)     │  │ Schema      │  │ Config      │  │   │
│  │  └─────────────┘  └─────────────┘  └─────────────┘  │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## 4. Agent 声明与网络注册

### 4.1 Agent 声明映射

Nexa 的 `agent` 声明自动映射到 Nexa-net 的网络节点：

```nexa
// Nexa 源码
@limit(max_tokens=4096)
@timeout(seconds=30)
agent DocumentTranslator implements TranslationProtocol uses PDFParser, TextProcessor {
    role: "Professional Document Translator",
    model: "claude-3.5-sonnet",
    prompt: "Translate documents while preserving formatting",
    
    // Nexa-net 特定配置
    nexa_net: {
        register: true,
        cost_per_page: 5,
        max_concurrent: 10,
        region: "asia-east"
    }
}
```

**生成的 Capability Schema：**

```yaml
nexa_capability:
  version: "1.0.0"
  metadata:
    name: "DocumentTranslator"
    description: "Professional Document Translator"
    tags: ["translation", "document"]
    
  endpoints:
    - id: "translate"
      name: "Document Translation"
      description: "Translate documents while preserving formatting"
      input_schema:
        type: object
        properties:
          document:
            type: binary
            format: application/pdf
          source_language:
            type: string
          target_language:
            type: string
        required: ["document", "source_language", "target_language"]
      output_schema:
        $ref: "#/components/schemas/TranslationProtocol"
      cost:
        model: "per_page"
        base_price: 5
      rate_limit:
        max_concurrent: 10
```

### 4.2 Tool 声明映射

```nexa
// 本地工具
tool Calculator {
    description: "Perform basic math operations",
    parameters: {"expression": "string"}
}

// Nexa-net 远程工具
tool RemoteTranslator {
    nexa_net: {
        intent: "translate text between languages",
        max_budget: 50,
        timeout_ms: 30000
    }
}

// MCP 工具
tool SearchMCP {
    mcp: "github.com/nexa-ai/search-mcp"
}
```

### 4.3 Protocol 声明映射

```nexa
// Nexa Protocol 声明
protocol TranslationResult {
    translated_text: "string",
    source_language: "string",
    target_language: "string",
    confidence: "number",
    pages_processed: "integer"
}
```

**生成的 JSON Schema：**

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "properties": {
    "translated_text": { "type": "string" },
    "source_language": { "type": "string" },
    "target_language": { "type": "string" },
    "confidence": { "type": "number" },
    "pages_processed": { "type": "integer" }
  },
  "required": ["translated_text", "source_language", "target_language", "confidence"]
}
```

---

## 5. DAG 操作符与网络拓扑

### 5.1 分叉操作符 (`|>>`)

```nexa
// Nexa 代码
results = input |>> [Researcher, Analyst, Writer];
```

**网络执行流程：**

```
┌─────────────────────────────────────────────────────────────┐
│                    Fan-out Execution                        │
│                                                             │
│  ┌─────────┐                                                │
│  │  Input  │                                                │
│  └─────────┘                                                │
│       │                                                     │
│       │ Nexa-Proxy 并行调用                                  │
│       ▼                                                     │
│  ┌─────────────────────────────────────────────────────┐    │
│  │              Nexa-net Semantic Router               │    │
│  │  查询: intent="research", "analyze", "write"        │    │
│  └─────────────────────────────────────────────────────┘    │
│       │                                                     │
│       ├──────────────────┬──────────────────┐               │
│       ▼                  ▼                  ▼               │
│  ┌─────────┐        ┌─────────┐        ┌─────────┐         │
│  │Researcher│       │ Analyst │        │ Writer  │         │
│  │(Remote) │        │(Remote) │        │(Remote) │         │
│  └─────────┘        └─────────┘        └─────────┘         │
│       │                  │                  │               │
│       ▼                  ▼                  ▼               │
│  [result1,           result2,           result3]            │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### 5.2 合流操作符 (`&>>`)

```nexa
// Nexa 代码
report = [Researcher, Analyst] &>> Reviewer;
```

**网络执行流程：**

```
┌─────────────────────────────────────────────────────────────┐
│                    Fan-in Execution                         │
│                                                             │
│  ┌─────────┐        ┌─────────┐                            │
│  │Researcher│       │ Analyst │                            │
│  └─────────┘        └─────────┘                            │
│       │                  │                                  │
│       ▼                  ▼                                  │
│  [result1,           result2]                               │
│       │                  │                                  │
│       └────────┬─────────┘                                  │
│                │                                            │
│                ▼                                            │
│  ┌─────────────────────────────────────────────────────┐    │
│  │              Nexa-net Semantic Router               │    │
│  │  查询: intent="review and consolidate"              │    │
│  └─────────────────────────────────────────────────────┘    │
│                │                                            │
│                ▼                                            │
│           ┌─────────┐                                       │
│           │Reviewer │                                       │
│           │(Remote) │                                       │
│           └─────────┘                                       │
│                │                                            │
│                ▼                                            │
│           [report]                                          │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### 5.3 条件分支操作符 (`??`)

```nexa
// Nexa 代码
result = input ?? UrgentHandler : NormalHandler;
```

**网络执行流程：**

```
┌─────────────────────────────────────────────────────────────┐
│                    Conditional Branch                       │
│                                                             │
│  ┌─────────┐                                                │
│  │  Input  │                                                │
│  └─────────┘                                                │
│       │                                                     │
│       ▼                                                     │
│  ┌─────────────────────────────────────────────────────┐    │
│  │              Semantic Condition Check               │    │
│  │  semantic_if "is urgent" against input              │    │
│  └─────────────────────────────────────────────────────┘    │
│       │                                                     │
│       ├──────────────────┬──────────────────┐               │
│       │ true             │ false            │               │
│       ▼                  ▼                  │               │
│  ┌─────────────┐    ┌─────────────┐        │               │
│  │UrgentHandler│    │NormalHandler│        │               │
│  │ (Remote)    │    │ (Remote)    │        │               │
│  └─────────────┘    └─────────────┘        │               │
│       │                  │                  │               │
│       └────────┬─────────┘                  │               │
│                ▼                                            │
│           [result]                                          │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### 5.4 DAG 操作符网络映射表

| Nexa 操作符 | 网络操作 | 描述 |
|-------------|----------|------|
| `A >> B` | 顺序调用 | A 的输出作为 B 的输入 |
| `A \|>> [B, C]` | 并行调用 | A 的输出并行发送到 B、C |
| `[A, B] &>> C` | 合流调用 | A、B 的输出合并发送到 C |
| `A ?? B : C` | 条件调用 | 根据条件选择 B 或 C |
| `A \|\| [B, C]` | Fire-forget | 异步调用，不等待结果 |
| `[A, B] && C` | 共识调用 | 需要 A、B 达成一致后调用 C |

---

## 6. 语义路由集成

### 6.1 意图路由映射

```nexa
// Nexa 代码
match user_request {
    intent("查询天气") => WeatherBot.run(user_request),
    intent("查询股市") => StockBot.run(user_request),
    intent("翻译文档") => Translator.run(user_request),
    _ => GeneralBot.run(user_request)
}
```

**网络执行：**

```python
# 生成的网络调用
async def route_intent(user_request: str) -> str:
    # 1. 向量化意图
    intent_vector = await nexa_proxy.vectorize(user_request)
    
    # 2. 查询语义路由表
    candidates = await nexa_proxy.route_query(
        intent_vector=intent_vector,
        top_k=3
    )
    
    # 3. 选择最佳匹配
    best_match = candidates[0]
    
    # 4. 调用远程 Agent
    result = await nexa_proxy.call(
        target_did=best_match.provider_did,
        endpoint_id=best_match.endpoint_id,
        data={"request": user_request}
    )
    
    return result
```

### 6.2 语义条件集成

```nexa
// Nexa 代码
semantic_if "包含具体的日期和地点" fast_match r"\d{4}-\d{2}-\d{2}" against user_input {
    schedule_tool.run(user_input);
} else {
    clarification_agent.run(user_input);
}
```

**网络执行：**

```python
# 生成的网络调用
async def semantic_condition(user_input: str) -> str:
    # 1. Fast-path: 本地正则匹配
    import re
    if re.search(r"\d{4}-\d{2}-\d{2}", user_input):
        # 2. 调用远程工具
        result = await nexa_proxy.call(
            intent="schedule appointment",
            data={"input": user_input}
        )
        return result
    
    # 3. Fallback: 语义判断
    semantic_result = await nexa_proxy.semantic_check(
        condition="包含具体的日期和地点",
        data=user_input
    )
    
    if semantic_result.matched:
        result = await nexa_proxy.call(
            intent="schedule appointment",
            data={"input": user_input}
        )
    else:
        result = await nexa_proxy.call(
            intent="clarify request",
            data={"input": user_input}
        )
    
    return result
```

---

## 7. AVM 作为执行引擎

### 7.1 AVM 架构

```
┌─────────────────────────────────────────────────────────────┐
│                    AVM Architecture                         │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Compiler Frontend                       │   │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐          │   │
│  │  │  Lexer   │─▶│  Parser  │─▶│Type Check│          │   │
│  │  │ (logos)  │  │ (递归下降)│  │          │          │   │
│  │  └──────────┘  └──────────┘  └──────────┘          │   │
│  └─────────────────────────────────────────────────────┘   │
│                           │                                 │
│                           ▼                                 │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Bytecode Compiler                      │   │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐          │   │
│  │  │   AST    │─▶│ Bytecode │─▶│  Module  │          │   │
│  │  │          │  │ Generator│  │ (.nxbc)  │          │   │
│  │  └──────────┘  └──────────┘  └──────────┘          │   │
│  └─────────────────────────────────────────────────────┘   │
│                           │                                 │
│                           ▼                                 │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              AVM Runtime                            │   │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐          │   │
│  │  │Interpreter│ │ Scheduler │ │Agent Reg │          │   │
│  │  │  (栈式)   │ │ (Tokio)  │ │          │          │   │
│  │  └──────────┘  └──────────┘  └──────────┘          │   │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐          │   │
│  │  │LLM Client│ │Tool Exec │ │ContextPager│         │   │
│  │  └──────────┘  └──────────┘  └──────────┘          │   │
│  └─────────────────────────────────────────────────────┘   │
│                           │                                 │
│                           ▼                                 │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Nexa-net Integration                   │   │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐          │   │
│  │  │Network API│ │DID Manager│ │Channel Mgr│         │   │
│  │  └──────────┘  └──────────┘  └──────────┘          │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### 7.2 AVM 与 Nexa-Proxy 集成

```rust
// avm/src/runtime/nexa_net.rs

use crate::utils::error::{AvmError, AvmResult};

/// Nexa-net 网络客户端
pub struct NexaNetClient {
    proxy_address: String,
    did: String,
    channel_manager: ChannelManager,
}

impl NexaNetClient {
    /// 调用远程 Agent
    pub async fn call_agent(
        &self,
        intent: &str,
        data: &[u8],
        budget: u64,
    ) -> AvmResult<Vec<u8>> {
        // 1. 语义路由
        let candidates = self.route(intent).await?;
        
        // 2. 选择最佳候选
        let target = candidates.into_iter()
            .next()
            .ok_or(AvmError::NoMatchingService)?;
        
        // 3. 建立连接
        let conn = self.connect(&target.did).await?;
        
        // 4. 执行 RPC
        let result = self.rpc_call(conn, &target.endpoint_id, data, budget).await?;
        
        Ok(result)
    }
    
    /// 注册本地 Agent 到网络
    pub async fn register_agent(
        &self,
        name: &str,
        schema: CapabilitySchema,
    ) -> AvmResult<()> {
        self.register_capability(name, schema).await
    }
}
```

### 7.3 智能调度器集成

```rust
// avm/src/vm/scheduler.rs

/// 智能调度器
pub struct SmartScheduler {
    strategy: ScheduleStrategy,
    priority_queue: PriorityQueue<Task>,
    load_monitor: LoadMonitor,
}

#[derive(Clone, Copy)]
pub enum ScheduleStrategy {
    RoundRobin,
    LeastLoaded,
    Adaptive,
}

impl SmartScheduler {
    /// 调度任务到最佳节点
    pub async fn schedule(&self, task: Task) -> AvmResult<ScheduledTask> {
        match self.strategy {
            ScheduleStrategy::RoundRobin => {
                // 轮询调度
                let node = self.round_robin_select()?;
                Ok(ScheduledTask { task, node })
            }
            ScheduleStrategy::LeastLoaded => {
                // 最小负载调度
                let node = self.least_loaded_select()?;
                Ok(ScheduledTask { task, node })
            }
            ScheduleStrategy::Adaptive => {
                // 自适应调度（考虑网络延迟、成本、负载）
                let node = self.adaptive_select(&task).await?;
                Ok(ScheduledTask { task, node })
            }
        }
    }
}
```

### 7.4 向量虚存分页集成

```rust
// avm/src/vm/context_pager.rs

/// 向量虚存分页器
pub struct ContextPager {
    pages: HashMap<PageId, MemoryPage>,
    embedding_model: EmbeddingModel,
    eviction_policy: EvictionPolicy,
}

impl ContextPager {
    /// 语义相关性加载
    pub async fn load_relevant_pages(
        &mut self,
        query: &str,
    ) -> AvmResult<Vec<MemoryPage>> {
        // 1. 向量化查询
        let query_vector = self.embedding_model.embed(query)?;
        
        // 2. 计算页面相似度
        let mut scores: Vec<(PageId, f32)> = self.pages.iter()
            .map(|(id, page)| {
                let similarity = cosine_similarity(&query_vector, &page.embedding);
                (*id, similarity)
            })
            .collect();
        
        // 3. 排序并返回最相关页面
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        
        let relevant: Vec<MemoryPage> = scores.iter()
            .take(10)
            .filter_map(|(id, _)| self.pages.get(id).cloned())
            .collect();
        
        Ok(relevant)
    }
}
```

---

## 8. 经济模型集成

### 8.1 装饰器与预算控制

```nexa
// Nexa 代码
@limit(max_tokens=4096)
@timeout(seconds=30)
@budget(max_cost=100)  // Nexa-net 预算控制
@retry(max_attempts=3, backoff="exponential")
agent PremiumTranslator {
    model: "gpt-4",
    prompt: "Professional translation service"
}
```

**生成的网络配置：**

```yaml
agent_config:
  name: "PremiumTranslator"
  budget:
    max_cost: 100  # NEXA
    per_call_limit: 10
  timeout: 30s
  retry:
    max_attempts: 3
    backoff: exponential
    initial_delay: 1s
```

### 8.2 微交易集成

```nexa
// Nexa 代码
flow paid_service {
    // 设置本次调用的预算
    @budget(50)
    result = RemoteAnalyzer.run(data);
    
    // 检查实际花费
    cost = get_cost(result);
    print(f"Service cost: {cost} NEXA");
}
```

**网络执行：**

```python
async def paid_service(data: bytes) -> Result:
    # 1. 检查通道余额
    balance = await nexa_proxy.get_balance()
    if balance < 50:
        raise InsufficientBalanceError()
    
    # 2. 发起调用（带预算）
    result = await nexa_proxy.call(
        intent="analyze data",
        data=data,
        budget=50
    )
    
    # 3. 获取实际花费
    cost = result.cost
    
    return result
```

---

## 9. 安全沙盒集成

### 9.1 WASM 沙盒

```rust
// avm/src/wasm/sandbox.rs

/// WASM 沙盒执行器
pub struct WasmSandbox {
    engine: wasmtime::Engine,
    store: wasmtime::Store<SandboxState>,
    permissions: PermissionLevel,
}

#[derive(Clone, Copy, PartialEq)]
pub enum PermissionLevel {
    None,       // 无权限
    Standard,   // 标准权限（文件读取、网络请求）
    Elevated,   // 提升权限（文件写入、子进程）
    Full,       // 完全权限
}

impl WasmSandbox {
    /// 在沙盒中执行工具
    pub async fn execute_tool(
        &mut self,
        wasm_module: &[u8],
        function: &str,
        args: &[u8],
    ) -> AvmResult<Vec<u8>> {
        // 1. 加载 WASM 模块
        let module = wasmtime::Module::new(&self.engine, wasm_module)?;
        
        // 2. 创建实例（受限环境）
        let instance = wasmtime::Instance::new(&mut self.store, &module, &[])?;
        
        // 3. 获取导出函数
        let func = instance.get_typed_func::<(i32, i32), i32>(&mut self.store, function)?;
        
        // 4. 执行（带资源限制）
        let result = func.call(&mut self.store, (args.as_ptr() as i32, args.len() as i32))?;
        
        Ok(self.extract_result(result))
    }
}
```

### 9.2 工具执行安全

```nexa
// Nexa 代码
tool UnsafeTool {
    description: "Execute shell commands",
    parameters: {"command": "string"},
    
    // 安全配置
    sandbox: {
        enabled: true,
        permissions: "standard",
        timeout_ms: 5000,
        memory_limit_mb: 64
    }
}
```

---

## 10. 开发者体验

### 10.1 完整示例

```nexa
// translator_service.nx

// 定义协议
protocol TranslationResult {
    translated_text: "string",
    source_language: "string",
    target_language: "string",
    confidence: "number"
}

// 定义本地工具
tool PDFParser {
    description: "Parse PDF documents",
    parameters: {"file": "binary"}
}

// 定义远程工具（通过 Nexa-net）
tool RemoteReviewer {
    nexa_net: {
        intent: "review translation quality",
        max_budget: 20
    }
}

// 定义 Agent
@budget(max_cost=100)
agent DocumentTranslator implements TranslationResult uses PDFParser, RemoteReviewer {
    role: "Professional Document Translator",
    model: "claude-3.5-sonnet",
    prompt: "Translate documents while preserving formatting",
    
    nexa_net: {
        register: true,
        cost_per_page: 5,
        region: "asia-east"
    }
}

// 定义流程
flow main {
    // 读取文档
    document = PDFParser.run(input_file);
    
    // 并行翻译到多种语言
    translations = document |>> [
        DocumentTranslator.run(target="zh"),
        DocumentTranslator.run(target="ja"),
        DocumentTranslator.run(target="ko")
    ];
    
    // 合流审核
    reviewed = translations &>> RemoteReviewer;
    
    // 返回结果
    return reviewed;
}

// 定义测试
test "translation_quality" {
    result = DocumentTranslator.run("Hello, World!", target="zh");
    assert "包含正确的中文翻译" against result;
}
```

### 10.2 编译与部署

```bash
# 编译 Nexa 代码
nexa build translator_service.nx

# 输出：
# ✓ Compiled to AVM bytecode: translator_service.nxbc
# ✓ Generated capability schema: translator_service.schema.yaml
# ✓ Generated network config: translator_service.netconf.yaml

# 部署到 Nexa-net
nexa deploy translator_service.nxbc --network nexa-mainnet

# 输出：
# ✓ Registered DID: did:nexa:translator123...
# ✓ Published capability to DHT
# ✓ Opened payment channel
# ✓ Service is now discoverable on Nexa-net
```

### 10.3 调用远程服务

```nexa
// client.nx

// 定义远程服务调用
tool TranslationService {
    nexa_net: {
        intent: "translate documents professionally",
        max_budget: 50,
        timeout_ms: 30000,
        preferred_region: "asia-east"
    }
}

flow client_flow {
    // 调用远程翻译服务
    result = TranslationService.run(
        document=my_document,
        source="en",
        target="zh"
    );
    
    print(f"Translation cost: {result.cost} NEXA");
    print(f"Translated by: {result.provider_did}");
    
    return result;
}
```

---

## 11. 相关文档

### Nexa-net 文档

- [整体架构设计](./ARCHITECTURE.md) - 四层架构总览
- [语义发现与能力路由层](./DISCOVERY_LAYER.md) - 语义路由详细设计
- [传输与协商协议层](./TRANSPORT_LAYER.md) - RPC 协议设计
- [资源管理与微交易层](./ECONOMY_LAYER.md) - 经济模型设计
- [开发者接入指南](./DEVELOPER_GUIDE.md) - SDK 使用指南

### Nexa 语言文档

- [Nexa 语法参考](https://ouyangyipeng.github.io/Nexa-docs/) - 完整语法手册
- [Nexa 编译器架构](https://github.com/ouyangyipeng/Nexa/blob/main/docs/02_compiler_architecture.md) - 编译器设计
- [Nexa 路线图](https://github.com/ouyangyipeng/Nexa/blob/main/docs/03_roadmap_and_vision.md) - 发展规划

### 参考资料

- [Nexa GitHub](https://github.com/ouyangyipeng/Nexa)
- [Nexa-docs](https://ouyangyipeng.github.io/Nexa-docs/)
- [Model Context Protocol](https://modelcontextprotocol.io/)