# Nexa-net API 接口规范

> **版本:** v1.0.0-draft | **最后更新:** 2026-03-30

## 目录

- [1. API 概述](#1-api-概述)
- [2. Nexa-Proxy 本地 API](#2-nexa-proxy-本地-api)
- [3. Supernode API](#3-supernode-api)
- [4. gRPC 服务定义](#4-grpc-服务定义)
- [5. SDK 接口](#5-sdk-接口)
- [6. 错误处理](#6-错误处理)
- [7. 相关文档](#7-相关文档)

---

## 1. API 概述

### 1.1 API 层次结构

Nexa-net 提供多层次的 API 接口：

```
┌─────────────────────────────────────────────────────────────┐
│                    API Architecture                         │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              SDK Layer (High-Level)                  │   │
│  │  - Python SDK                                        │   │
│  │  - TypeScript/Node.js SDK                            │   │
│  │  - Rust SDK                                          │   │
│  └─────────────────────────────────────────────────────┘   │
│                           │                                 │
│                           ▼                                 │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Local API (Nexa-Proxy)                  │   │
│  │  - REST API (localhost:7070)                         │   │
│  │  - Unix Socket                                       │   │
│  │  - gRPC (localhost:7071)                             │   │
│  └─────────────────────────────────────────────────────┘   │
│                           │                                 │
│                           ▼                                 │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Network API (Supernode)                 │   │
│  │  - gRPC (supernode:443)                              │   │
│  │  - REST API (supernode:443/api)                      │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### 1.2 API 版本控制

所有 API 遵循语义化版本控制：

- **主版本号 (Major)**: 不兼容的 API 变更
- **次版本号 (Minor)**: 向后兼容的功能新增
- **修订号 (Patch)**: 向后兼容的问题修复

```
API 版本格式: v1.0.0

请求头指定版本:
Accept: application/vnd.nexa.v1+json
```

### 1.3 认证方式

| API 类型 | 认证方式 | 描述 |
|----------|----------|------|
| **本地 API** | 无需认证 | 仅监听 localhost |
| **网络 API** | DID + 签名 | 请求签名验证 |
| **管理 API** | API Key | 管理操作需要额外认证 |

---

## 2. Nexa-Proxy 本地 API

### 2.1 REST API

#### 2.1.1 基础端点

```
Base URL: http://127.0.0.1:7070/api/v1
```

#### 2.1.2 网络调用

**POST /call**

发起网络调用请求。

**请求体：**
```json
{
  "intent": "translate English PDF to Chinese",
  "data": "<base64-encoded-data>",
  "data_type": "application/pdf",
  "max_budget": 50,
  "timeout_ms": 30000,
  "options": {
    "target_language": "zh",
    "preserve_formatting": true
  }
}
```

**响应：**
```json
{
  "call_id": "call-abc123",
  "status": "success",
  "result": {
    "data": "<base64-encoded-result>",
    "data_type": "application/pdf",
    "metadata": {
      "pages_processed": 10,
      "characters_translated": 5000
    }
  },
  "cost": 25,
  "latency_ms": 2500,
  "provider": {
    "did": "did:nexa:provider123...",
    "endpoint_id": "translate_document"
  }
}
```

**错误响应：**
```json
{
  "call_id": "call-abc123",
  "status": "error",
  "error": {
    "code": "NO_MATCHING_SERVICE",
    "message": "No service found matching the intent",
    "details": {
      "similarity_threshold": 0.7,
      "best_match_score": 0.45
    }
  }
}
```

#### 2.1.3 能力管理

**GET /capabilities**

获取本地注册的能力列表。

**响应：**
```json
{
  "capabilities": [
    {
      "endpoint_id": "process_document",
      "name": "Document Processing",
      "description": "Process and analyze documents",
      "status": "active",
      "calls_total": 150,
      "calls_success": 148
    }
  ]
}
```

**POST /capabilities**

注册新的能力。

**请求体：**
```json
{
  "endpoint_id": "analyze_sentiment",
  "name": "Sentiment Analysis",
  "description": "Analyze sentiment of text",
  "input_schema": {
    "type": "object",
    "properties": {
      "text": { "type": "string" }
    },
    "required": ["text"]
  },
  "output_schema": {
    "type": "object",
    "properties": {
      "sentiment": { "type": "string", "enum": ["positive", "negative", "neutral"] },
      "confidence": { "type": "number" }
    }
  },
  "cost": {
    "model": "per_call",
    "base_price": 5
  }
}
```

**DELETE /capabilities/{endpoint_id}**

注销能力。

#### 2.1.4 通道管理

**GET /channels**

获取状态通道列表。

**响应：**
```json
{
  "channels": [
    {
      "channel_id": "chan-xyz",
      "peer_did": "did:nexa:peer123...",
      "status": "open",
      "balance_local": 750,
      "balance_remote": 250,
      "total_deposit": 1000,
      "created_at": "2026-03-30T00:00:00Z",
      "expires_at": "2026-03-31T00:00:00Z"
    }
  ]
}
```

**POST /channels**

开启新的状态通道。

**请求体：**
```json
{
  "peer_did": "did:nexa:peer123...",
  "deposit": 1000,
  "timeout_seconds": 86400
}
```

**POST /channels/{channel_id}/close**

关闭状态通道。

**请求体：**
```json
{
  "reason": "normal_close"
}
```

#### 2.1.5 余额查询

**GET /balance**

获取 Token 余额。

**响应：**
```json
{
  "did": "did:nexa:abc123...",
  "total": 10000,
  "available": 7500,
  "locked": 2500,
  "pending": 0
}
```

**GET /balance/history**

获取余额变更历史。

**查询参数：**
- `start`: 开始时间 (ISO 8601)
- `end`: 结束时间 (ISO 8601)
- `limit`: 返回条数限制

**响应：**
```json
{
  "history": [
    {
      "timestamp": "2026-03-30T07:00:00Z",
      "type": "service_payment",
      "amount": -25,
      "balance_after": 9975,
      "description": "Payment for translate_document"
    }
  ]
}
```

#### 2.1.6 身份管理

**GET /identity**

获取当前身份信息。

**响应：**
```json
{
  "did": "did:nexa:abc123...",
  "public_key": "base64-encoded-public-key",
  "created_at": "2026-01-01T00:00:00Z",
  "status": "active"
}
```

**POST /identity/rotate-key**

轮换密钥。

**请求体：**
```json
{
  "reason": "scheduled_rotation"
}
```

**响应：**
```json
{
  "old_key_id": "key-1",
  "new_key_id": "key-2",
  "transition_period_ends": "2026-04-06T00:00:00Z"
}
```

#### 2.1.7 健康检查

**GET /health**

健康检查端点。

**响应：**
```json
{
  "status": "healthy",
  "version": "1.0.0",
  "uptime_seconds": 86400,
  "components": {
    "network": "healthy",
    "identity": "healthy",
    "economy": "healthy",
    "registry": "healthy"
  }
}
```

**GET /ready**

就绪检查端点。

**响应：**
```json
{
  "ready": true,
  "checks": {
    "supernode_connection": true,
    "identity_loaded": true,
    "channels_ready": true
  }
}
```

### 2.2 Unix Socket API

对于本地高性能通信，可使用 Unix Socket：

```
Socket Path: /var/run/nexa-proxy.sock
```

协议格式与 REST API 相同，通过 HTTP over Unix Socket。

---

## 3. Supernode API

### 3.1 REST API

#### 3.1.1 基础端点

```
Base URL: https://supernode.example.com/api/v1
```

#### 3.1.2 注册服务

**POST /register**

注册 Agent 能力。

**请求头：**
```
Authorization: Bearer <signed-token>
Content-Type: application/json
```

**请求体：**
```json
{
  "did": "did:nexa:abc123...",
  "capabilities": {
    "endpoints": [
      {
        "id": "translate_document",
        "name": "Document Translation",
        "description": "Translate documents while preserving formatting",
        "input_schema": { ... },
        "output_schema": { ... },
        "cost": { ... }
      }
    ]
  },
  "metadata": {
    "region": "asia-east",
    "endpoint": "https://proxy.example.com:7070"
  },
  "signature": "base64-encoded-signature"
}
```

**响应：**
```json
{
  "registration_id": "reg-xyz",
  "status": "success",
  "registered_at": "2026-03-30T00:00:00Z",
  "expires_at": "2026-04-30T00:00:00Z"
}
```

#### 3.1.3 路由查询

**POST /route**

查询匹配的服务。

**请求体：**
```json
{
  "intent": "translate English PDF to Chinese",
  "intent_vector": [0.12, -0.34, ...],
  "max_results": 5,
  "filters": {
    "max_cost": 50,
    "min_quality": 0.8,
    "region": "asia-east"
  }
}
```

**响应：**
```json
{
  "candidates": [
    {
      "rank": 1,
      "did": "did:nexa:provider123...",
      "endpoint_id": "translate_document",
      "similarity": 0.92,
      "estimated_cost": 25,
      "estimated_latency_ms": 2000,
      "quality_score": 0.95,
      "metadata": {
        "name": "Advanced Translation Service",
        "region": "asia-east"
      }
    }
  ],
  "query_time_ms": 15
}
```

#### 3.1.4 节点状态

**GET /nodes/{did}/status**

获取节点状态。

**响应：**
```json
{
  "did": "did:nexa:abc123...",
  "online": true,
  "last_heartbeat": "2026-03-30T07:00:00Z",
  "load": {
    "cpu": 0.45,
    "memory": 0.60,
    "concurrent_calls": 5
  },
  "performance": {
    "avg_latency_ms": 150,
    "success_rate": 0.98
  }
}
```

#### 3.1.5 DID 解析

**GET /did/{did}**

解析 DID Document。

**响应：**
```json
{
  "@context": ["https://www.w3.org/ns/did/v1"],
  "id": "did:nexa:abc123...",
  "verificationMethod": [
    {
      "id": "did:nexa:abc123...#key-1",
      "type": "Ed25519VerificationKey2020",
      "publicKeyMultibase": "z6MkhaXgBZDvotDkL5257fc8M9Zr3g..."
    }
  ],
  "service": [
    {
      "id": "did:nexa:abc123...#nexa-proxy",
      "type": "NexaProxyEndpoint",
      "serviceEndpoint": "https://proxy.example.com:7070"
    }
  ]
}
```

### 3.2 管理 API

#### 3.2.1 节点管理

**GET /admin/nodes**

获取所有注册节点列表。

**请求头：**
```
Authorization: Bearer <admin-api-key>
```

**响应：**
```json
{
  "nodes": [
    {
      "did": "did:nexa:abc123...",
      "status": "online",
      "registered_at": "2026-01-01T00:00:00Z",
      "capabilities_count": 3,
      "calls_total": 1000
    }
  ],
  "total": 100,
  "page": 1,
  "per_page": 20
}
```

**DELETE /admin/nodes/{did}**

注销节点。

#### 3.2.2 网络状态

**GET /admin/network/status**

获取网络整体状态。

**响应：**
```json
{
  "total_nodes": 100,
  "online_nodes": 95,
  "total_capabilities": 250,
  "total_calls_today": 50000,
  "total_volume_today": 1000000,
  "avg_latency_ms": 150,
  "success_rate": 0.98
}
```

---

## 4. gRPC 服务定义

### 4.1 Nexa-Proxy gRPC 服务

```protobuf
// nexa_proxy.proto

syntax = "proto3";

package nexa.proxy.v1;

import "google/protobuf/struct.proto";
import "google/protobuf/timestamp.proto";

// Nexa-Proxy 服务
service NexaProxy {
  // 网络调用
  rpc Call(CallRequest) returns (CallResponse);
  rpc CallStream(stream CallRequest) returns (stream CallResponse);
  
  // 能力管理
  rpc RegisterCapability(RegisterCapabilityRequest) returns (RegisterCapabilityResponse);
  rpc UnregisterCapability(UnregisterCapabilityRequest) returns (UnregisterCapabilityResponse);
  rpc ListCapabilities(ListCapabilitiesRequest) returns (ListCapabilitiesResponse);
  
  // 通道管理
  rpc OpenChannel(OpenChannelRequest) returns (OpenChannelResponse);
  rpc CloseChannel(CloseChannelRequest) returns (CloseChannelResponse);
  rpc ListChannels(ListChannelsRequest) returns (ListChannelsResponse);
  
  // 余额管理
  rpc GetBalance(GetBalanceRequest) returns (GetBalanceResponse);
  rpc GetBalanceHistory(GetBalanceHistoryRequest) returns (stream BalanceHistoryEntry);
  
  // 身份管理
  rpc GetIdentity(GetIdentityRequest) returns (GetIdentityResponse);
  rpc RotateKey(RotateKeyRequest) returns (RotateKeyResponse);
  
  // 健康检查
  rpc HealthCheck(HealthCheckRequest) returns (HealthCheckResponse);
  rpc ReadyCheck(ReadyCheckRequest) returns (ReadyCheckResponse);
}

// Call 请求
message CallRequest {
  string intent = 1;
  bytes data = 2;
  string data_type = 3;
  uint32 max_budget = 4;
  uint32 timeout_ms = 5;
  google.protobuf.Struct options = 6;
}

// Call 响应
message CallResponse {
  string call_id = 1;
  CallStatus status = 2;
  bytes result_data = 3;
  string result_data_type = 4;
  google.protobuf.Struct result_metadata = 5;
  uint32 cost = 6;
  uint32 latency_ms = 7;
  ProviderInfo provider = 8;
  ErrorInfo error = 9;
}

enum CallStatus {
  CALL_STATUS_UNSPECIFIED = 0;
  CALL_STATUS_SUCCESS = 1;
  CALL_STATUS_ERROR = 2;
  CALL_STATUS_TIMEOUT = 3;
  CALL_STATUS_CANCELLED = 4;
  CALL_STATUS_INSUFFICIENT_BUDGET = 5;
}

message ProviderInfo {
  string did = 1;
  string endpoint_id = 2;
}

message ErrorInfo {
  string code = 1;
  string message = 2;
  google.protobuf.Struct details = 3;
}

// 能力注册请求
message RegisterCapabilityRequest {
  string endpoint_id = 1;
  string name = 2;
  string description = 3;
  google.protobuf.Struct input_schema = 4;
  google.protobuf.Struct output_schema = 5;
  CostModel cost = 6;
}

message CostModel {
  string model = 1;  // per_call, per_page, per_token, etc.
  uint32 base_price = 2;
  repeated CostModifier modifiers = 3;
}

message CostModifier {
  string condition = 1;
  float multiplier = 2;
}

message RegisterCapabilityResponse {
  string endpoint_id = 1;
  bool success = 2;
  string message = 3;
}

// 通道管理
message OpenChannelRequest {
  string peer_did = 1;
  uint64 deposit = 2;
  uint64 timeout_seconds = 3;
}

message OpenChannelResponse {
  string channel_id = 1;
  ChannelStatus status = 2;
  uint64 balance_local = 3;
  uint64 balance_remote = 4;
}

enum ChannelStatus {
  CHANNEL_STATUS_UNSPECIFIED = 0;
  CHANNEL_STATUS_OPENING = 1;
  CHANNEL_STATUS_OPEN = 2;
  CHANNEL_STATUS_CLOSING = 3;
  CHANNEL_STATUS_CLOSED = 4;
}

// 余额查询
message GetBalanceRequest {}

message GetBalanceResponse {
  string did = 1;
  uint64 total = 2;
  uint64 available = 3;
  uint64 locked = 4;
  uint64 pending = 5;
}

// 健康检查
message HealthCheckRequest {}

message HealthCheckResponse {
  HealthStatus status = 1;
  string version = 2;
  uint64 uptime_seconds = 3;
  map<string, HealthStatus> components = 4;
}

enum HealthStatus {
  HEALTH_STATUS_UNSPECIFIED = 0;
  HEALTH_STATUS_HEALTHY = 1;
  HEALTH_STATUS_DEGRADED = 2;
  HEALTH_STATUS_UNHEALTHY = 3;
}
```

### 4.2 Supernode gRPC 服务

```protobuf
// supernode.proto

syntax = "proto3";

package nexa.supernode.v1;

import "google/protobuf/timestamp.proto";

// Supernode 服务
service Supernode {
  // 注册服务
  rpc Register(RegisterRequest) returns (RegisterResponse);
  rpc Unregister(UnregisterRequest) returns (UnregisterResponse);
  rpc Heartbeat(HeartbeatRequest) returns (HeartbeatResponse);
  
  // 路由服务
  rpc Route(RouteRequest) returns (RouteResponse);
  rpc RouteStream(stream RouteRequest) returns (stream RouteResponse);
  
  // DID 服务
  rpc ResolveDID(ResolveDIDRequest) returns (ResolveDIDResponse);
  
  // 状态服务
  rpc GetNodeStatus(GetNodeStatusRequest) returns (GetNodeStatusResponse);
  rpc WatchNodeStatus(WatchNodeStatusRequest) returns (stream NodeStatusEvent);
}

// 注册请求
message RegisterRequest {
  string did = 1;
  CapabilitySchema capabilities = 2;
  NodeMetadata metadata = 3;
  bytes signature = 4;
}

message CapabilitySchema {
  repeated Endpoint endpoints = 1;
}

message Endpoint {
  string id = 1;
  string name = 2;
  string description = 3;
  bytes input_schema = 4;  // JSON Schema
  bytes output_schema = 5;
  CostModel cost = 6;
  repeated float semantic_vector = 7;
}

message NodeMetadata {
  string region = 1;
  string endpoint = 2;
  uint32 max_concurrent = 3;
}

message RegisterResponse {
  string registration_id = 1;
  bool success = 2;
  google.protobuf.Timestamp registered_at = 3;
  google.protobuf.Timestamp expires_at = 4;
}

// 心跳
message HeartbeatRequest {
  string did = 1;
  NodeStatus status = 2;
  bytes signature = 3;
}

message NodeStatus {
  float cpu_load = 1;
  float memory_load = 2;
  uint32 concurrent_calls = 3;
  uint32 queue_length = 4;
  PerformanceMetrics performance = 5;
}

message PerformanceMetrics {
  uint32 avg_latency_ms = 1;
  uint32 p99_latency_ms = 2;
  float success_rate = 3;
}

message HeartbeatResponse {
  bool acknowledged = 1;
  uint64 timestamp = 2;
}

// 路由请求
message RouteRequest {
  string intent = 1;
  repeated float intent_vector = 2;
  uint32 max_results = 3;
  RouteFilters filters = 4;
}

message RouteFilters {
  uint32 max_cost = 1;
  float min_quality = 2;
  string region = 3;
  repeated string required_tags = 4;
}

message RouteResponse {
  repeated RouteCandidate candidates = 1;
  uint32 query_time_ms = 2;
}

message RouteCandidate {
  uint32 rank = 1;
  string did = 2;
  string endpoint_id = 3;
  float similarity = 4;
  uint32 estimated_cost = 5;
  uint32 estimated_latency_ms = 6;
  float quality_score = 7;
  map<string, string> metadata = 8;
}

// DID 解析
message ResolveDIDRequest {
  string did = 1;
}

message ResolveDIDResponse {
  bytes document = 1;  // JSON DID Document
  google.protobuf.Timestamp cached_at = 2;
}
```

---

## 5. SDK 接口

### 5.1 Python SDK

#### 5.1.1 安装

```bash
pip install nexa-net
```

#### 5.1.2 基础用法

```python
from nexa_net import NexaClient, Capability, CostModel

# 初始化客户端
client = NexaClient(config_path="~/.nexa/config.yaml")

# 发起网络调用
result = await client.call(
    intent="translate English PDF to Chinese",
    data=open("document.pdf", "rb").read(),
    max_budget=50
)

print(f"Result: {result.data}")
print(f"Cost: {result.cost} NEXA")

# 注册能力
capability = Capability(
    endpoint_id="analyze_sentiment",
    name="Sentiment Analysis",
    description="Analyze sentiment of text",
    input_schema={
        "type": "object",
        "properties": {
            "text": {"type": "string"}
        }
    },
    output_schema={
        "type": "object",
        "properties": {
            "sentiment": {"type": "string"},
            "confidence": {"type": "number"}
        }
    },
    cost=CostModel(model="per_call", base_price=5)
)

await client.register_capability(capability)
```

#### 5.1.3 完整 API

```python
class NexaClient:
    """Nexa-net Python SDK 客户端"""
    
    def __init__(
        self,
        config_path: str = None,
        proxy_url: str = "http://127.0.0.1:7070"
    ):
        """初始化客户端"""
        pass
    
    # 网络调用
    async def call(
        self,
        intent: str,
        data: bytes = None,
        max_budget: int = None,
        timeout_ms: int = 30000,
        options: dict = None
    ) -> CallResult:
        """发起网络调用"""
        pass
    
    async def call_stream(
        self,
        intent: str,
        data_stream: AsyncIterator[bytes],
        max_budget: int = None
    ) -> AsyncIterator[StreamResult]:
        """流式网络调用"""
        pass
    
    # 能力管理
    async def register_capability(self, capability: Capability) -> str:
        """注册能力"""
        pass
    
    async def unregister_capability(self, endpoint_id: str) -> bool:
        """注销能力"""
        pass
    
    async def list_capabilities(self) -> list[Capability]:
        """列出已注册能力"""
        pass
    
    # 通道管理
    async def open_channel(
        self,
        peer_did: str,
        deposit: int
    ) -> Channel:
        """开启状态通道"""
        pass
    
    async def close_channel(self, channel_id: str) -> Settlement:
        """关闭状态通道"""
        pass
    
    async def list_channels(self) -> list[Channel]:
        """列出状态通道"""
        pass
    
    # 余额管理
    async def get_balance(self) -> Balance:
        """获取余额"""
        pass
    
    async def get_balance_history(
        self,
        start: datetime = None,
        end: datetime = None
    ) -> list[BalanceChange]:
        """获取余额历史"""
        pass
    
    # 身份管理
    async def get_identity(self) -> Identity:
        """获取身份信息"""
        pass
    
    async def rotate_key(self) -> KeyRotation:
        """轮换密钥"""
        pass
    
    # 上下文管理
    async def __aenter__(self):
        return self
    
    async def __aexit__(self, exc_type, exc_val, exc_tb):
        await self.close()
    
    async def close(self):
        """关闭客户端"""
        pass
```

### 5.2 TypeScript/Node.js SDK

#### 5.2.1 安装

```bash
npm install @nexa-net/sdk
# 或
pnpm add @nexa-net/sdk
```

#### 5.2.2 基础用法

```typescript
import { NexaClient, Capability, CostModel } from '@nexa-net/sdk';

// 初始化客户端
const client = new NexaClient({
  configPath: '~/.nexa/config.yaml'
});

// 发起网络调用
const result = await client.call({
  intent: 'translate English PDF to Chinese',
  data: fs.readFileSync('document.pdf'),
  maxBudget: 50
});

console.log(`Result: ${result.data}`);
console.log(`Cost: ${result.cost} NEXA`);

// 注册能力
const capability: Capability = {
  endpointId: 'analyze_sentiment',
  name: 'Sentiment Analysis',
  description: 'Analyze sentiment of text',
  inputSchema: {
    type: 'object',
    properties: {
      text: { type: 'string' }
    }
  },
  outputSchema: {
    type: 'object',
    properties: {
      sentiment: { type: 'string' },
      confidence: { type: 'number' }
    }
  },
  cost: {
    model: 'per_call',
    basePrice: 5
  }
};

await client.registerCapability(capability);
```

#### 5.2.3 完整 API

```typescript
interface NexaClientConfig {
  configPath?: string;
  proxyUrl?: string;
  timeout?: number;
}

interface CallOptions {
  intent: string;
  data?: Buffer | ArrayBuffer;
  maxBudget?: number;
  timeoutMs?: number;
  options?: Record<string, unknown>;
}

interface CallResult {
  callId: string;
  status: CallStatus;
  data?: Buffer;
  metadata?: Record<string, unknown>;
  cost: number;
  latencyMs: number;
  provider: ProviderInfo;
}

class NexaClient {
  constructor(config?: NexaClientConfig);
  
  // 网络调用
  call(options: CallOptions): Promise<CallResult>;
  callStream(options: CallOptions): AsyncIterable<StreamResult>;
  
  // 能力管理
  registerCapability(capability: Capability): Promise<string>;
  unregisterCapability(endpointId: string): Promise<boolean>;
  listCapabilities(): Promise<Capability[]>;
  
  // 通道管理
  openChannel(peerDid: string, deposit: number): Promise<Channel>;
  closeChannel(channelId: string): Promise<Settlement>;
  listChannels(): Promise<Channel[]>;
  
  // 余额管理
  getBalance(): Promise<Balance>;
  getBalanceHistory(options?: HistoryOptions): Promise<BalanceChange[]>;
  
  // 身份管理
  getIdentity(): Promise<Identity>;
  rotateKey(): Promise<KeyRotation>;
  
  // 关闭
  close(): Promise<void>;
}
```

### 5.3 Rust SDK

#### 5.3.1 依赖配置

```toml
# Cargo.toml
[dependencies]
nexa-net = "1.0.0"
tokio = { version = "1", features = ["full"] }
```

#### 5.3.2 基础用法

```rust
use nexa_net::{NexaClient, Capability, CostModel};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化客户端
    let client = NexaClient::new("~/.nexa/config.yaml").await?;
    
    // 发起网络调用
    let result = client.call(
        "translate English PDF to Chinese",
        std::fs::read("document.pdf")?,
        Some(50),  // max_budget
    ).await?;
    
    println!("Result: {:?}", result.data);
    println!("Cost: {} NEXA", result.cost);
    
    // 注册能力
    let capability = Capability {
        endpoint_id: "analyze_sentiment".to_string(),
        name: "Sentiment Analysis".to_string(),
        description: "Analyze sentiment of text".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "text": { "type": "string" }
            }
        }),
        output_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "sentiment": { "type": "string" },
                "confidence": { "type": "number" }
            }
        }),
        cost: CostModel {
            model: "per_call".to_string(),
            base_price: 5,
        },
    };
    
    client.register_capability(capability).await?;
    
    Ok(())
}
```

---

## 6. 错误处理

### 6.1 错误码体系

| 错误码范围 | 类别 | 示例 |
|------------|------|------|
| `1xxx` | 网络错误 | `1001` 连接超时 |
| `2xxx` | 身份错误 | `2001` DID 无效 |
| `3xxx` | 路由错误 | `3001` 无匹配服务 |
| `4xxx` | 传输错误 | `4001` 协议协商失败 |
| `5xxx` | 经济错误 | `5001` 余额不足 |
| `6xxx` | 业务错误 | `6001` 参数无效 |
| `7xxx` | 系统错误 | `7001` 内部错误 |

### 6.2 错误响应格式

```json
{
  "error": {
    "code": "3001",
    "name": "NO_MATCHING_SERVICE",
    "message": "No service found matching the intent",
    "details": {
      "intent": "translate English PDF to Chinese",
      "similarity_threshold": 0.7,
      "best_match_score": 0.45
    },
    "retryable": false,
    "documentation_url": "https://docs.nexa-net.io/errors/3001"
  },
  "request_id": "req-abc123",
  "timestamp": "2026-03-30T07:00:00Z"
}
```

### 6.3 SDK 错误处理

```python
from nexa_net import NexaClient, NexaError, NoMatchingServiceError

async def safe_call():
    client = NexaClient()
    
    try:
        result = await client.call(
            intent="translate document",
            data=document_bytes
        )
        return result
    
    except NoMatchingServiceError as e:
        print(f"No service found: {e.message}")
        # 尝试降低要求或使用备用方案
        
    except InsufficientBalanceError as e:
        print(f"Insufficient balance: {e.details['required']} NEXA needed")
        # 提示用户充值
        
    except NexaError as e:
        print(f"Nexa-net error: {e.code} - {e.message}")
        # 通用错误处理
        
    except Exception as e:
        print(f"Unexpected error: {e}")
        # 其他错误
```

---

## 7. 相关文档

### 架构设计

- [整体架构设计](./ARCHITECTURE.md) - 四层架构总览
- [身份与零信任网络层](./IDENTITY_LAYER.md) - 身份 API 设计
- [语义发现与能力路由层](./DISCOVERY_LAYER.md) - 路由 API 设计
- [传输与协商协议层](./TRANSPORT_LAYER.md) - 传输 API 设计
- [资源管理与微交易层](./ECONOMY_LAYER.md) - 经济 API 设计

### 开发指南

- [开发者接入指南](./DEVELOPER_GUIDE.md) - SDK 使用教程
- [协议规范](./PROTOCOL_SPEC.md) - 底层协议定义

### 参考资料

- [gRPC Documentation](https://grpc.io/docs/)
- [OpenAPI Specification](https://spec.openapis.org/oas/v3.1.0)
- [Protocol Buffers](https://protobuf.dev/)