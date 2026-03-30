# Nexa-net 传输与协商协议层

> **版本:** v1.0.0-draft | **最后更新:** 2026-03-30
> **所属架构层:** Layer 3 - Transport & Contract Layer

## 目录

- [1. 概述](#1-概述)
- [2. 动态协议协商](#2-动态协议协商)
- [3. 结构化流式 RPC](#3-结构化流式-rpc)
- [4. 二进制序列化](#4-二进制序列化)
- [5. 连接管理](#5-连接管理)
- [6. 错误处理与重试](#6-错误处理与重试)
- [7. 实现规范](#7-实现规范)
- [8. 性能考量](#8-性能考量)
- [9. 相关文档](#9-相关文档)

---

## 1. 概述

### 1.1 设计背景

传统 HTTP/REST API 在 Agent 间通讯场景下存在以下问题：

| 问题 | HTTP/REST 方案 | 影响 |
|------|----------------|------|
| **数据格式** | 文本 JSON | 体积大，Token 消耗高 |
| **序列化效率** | JSON 解析 | 速度慢，CPU 消耗高 |
| **流式传输** | 需要额外实现 | 大数据传输困难 |
| **多路复用** | HTTP/1.1 不支持 | 连接数受限 |
| **协议协商** | 无标准化 | 版本兼容困难 |

### 1.2 设计目标

Nexa-net 传输层设计目标：

1. **高密度传输 (High-Density Transport)** - 二进制协议，体积缩小 60%-80%
2. **流式处理 (Streaming Processing)** - 支持大规模上下文传输
3. **多路复用 (Multiplexing)** - 单连接支持多个并发调用
4. **动态协商 (Dynamic Negotiation)** - 自动协商最优协议版本
5. **高效序列化 (Efficient Serialization)** - Protobuf/FlatBuffers，速度提升 10x+

### 1.3 层级架构

```
┌─────────────────────────────────────────────────────────────┐
│                   Layer 3: Transport Layer                  │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Protocol Negotiator                     │   │
│  │  - 版本协商                                          │   │
│  │  - 序列化格式选择                                    │   │
│  │  - 压缩算法协商                                      │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              RPC Engine                              │   │
│  │  - 方法调用                                          │   │
│  │  - 流式传输                                          │   │
│  │  - 多路复用                                          │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Serialization Engine                    │   │
│  │  - Protobuf 编解码                                   │   │
│  │  - FlatBuffers 编解码                                │   │
│  │  - Schema 管理                                       │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Connection Manager                      │   │
│  │  - 连接池                                            │   │
│  │  - 会话管理                                          │   │
│  │  - 心跳保活                                          │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Error Handler                           │   │
│  │  - 错误分类                                          │   │
│  │  - 重试策略                                          │   │
│  │  - 超时管理                                          │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

---

## 2. 动态协议协商

### 2.1 协商流程

Nexa-net 在建立连接的最初 50ms 内完成协议协商：

```
┌─────────────┐                    ┌─────────────┐
│  Nexa-Proxy │                    │  Nexa-Proxy │
│     (A)     │                    │     (B)     │
└──────┬──────┘                    └──────┬──────┘
       │                                  │
       │ 1. SYN-NEXA                      │
       │─────────────────────────────────▶│
       │                                  │
       │ {                                │
       │   intent_hash: "abc123",         │
       │   max_budget: 50,                │
       │   supported_protocols: [         │
       │     "nexa-rpc-v1",               │
       │     "grpc",                      │
       │     "flatbuffers"                │
       │   ],                             │
       │   supported_encodings: [         │
       │     "gzip",                      │
       │     "lz4",                       │
       │     "none"                       │
       │   ]                              │
       │ }                                │
       │                                  │
       │ 2. ACK-SCHEMA                    │
       │◀─────────────────────────────────│
       │                                  │
       │ {                                │
       │   selected_protocol: "nexa-rpc-v1",│
       │   selected_encoding: "lz4",      │
       │   schema_hash: "def456",         │
       │   compressed_schema: <binary>,   │
       │   estimated_cost: 25,            │
       │   estimated_latency_ms: 2000     │
       │ }                                │
       │                                  │
       │ 3. ACCEPT                         │
       │─────────────────────────────────▶│
       │                                  │
       │ {                                │
       │   session_id: "sess-xyz",        │
       │   ready: true                    │
       │ }                                │
       │                                  │
       │ 4. EXEC (RPC Call)               │
       │─────────────────────────────────▶│
       │                                  │
```

### 2.2 SYN-NEXA 消息

#### 2.2.1 消息结构

```protobuf
// nexa_protocol.proto

message SynNexa {
  // 意图哈希（用于路由验证）
  string intent_hash = 1;
  
  // 最大预算（Nexa-Tokens）
  uint32 max_budget = 2;
  
  // 支持的协议版本
  repeated string supported_protocols = 3;
  
  // 支持的编码格式
  repeated string supported_encodings = 4;
  
  // 支持的压缩算法
  repeated CompressionType supported_compressions = 5;
  
  // 客户端能力
  ClientCapabilities capabilities = 6;
  
  // 时间戳
  uint64 timestamp = 7;
  
  // 签名
  bytes signature = 8;
}

enum CompressionType {
  NONE = 0;
  GZIP = 1;
  LZ4 = 2;
  ZSTD = 3;
}

message ClientCapabilities {
  // 最大并发流数
  uint32 max_concurrent_streams = 1;
  
  // 最大消息大小
  uint64 max_message_size = 2;
  
  // 是否支持流式传输
  bool streaming = 3;
  
  // 是否支持双向流
  bool bidirectional = 4;
}
```

### 2.3 ACK-SCHEMA 消息

#### 2.3.1 消息结构

```protobuf
message AckSchema {
  // 选定的协议
  string selected_protocol = 1;
  
  // 选定的编码
  string selected_encoding = 2;
  
  // 选定的压缩算法
  CompressionType selected_compression = 3;
  
  // Schema 哈希
  string schema_hash = 4;
  
  // 压缩后的 Schema（二进制）
  bytes compressed_schema = 5;
  
  // 预估成本
  uint32 estimated_cost = 6;
  
  // 预估延迟（毫秒）
  uint32 estimated_latency_ms = 7;
  
  // 服务端能力
  ServerCapabilities capabilities = 8;
  
  // 时间戳
  uint64 timestamp = 9;
  
  // 签名
  bytes signature = 10;
}

message ServerCapabilities {
  // 最大并发流数
  uint32 max_concurrent_streams = 1;
  
  // 最大消息大小
  uint64 max_message_size = 2;
  
  // 当前负载
  float current_load = 3;
  
  // 可用队列位置
  uint32 available_queue_slots = 4;
}
```

### 2.4 协商算法

#### 2.4.1 协议选择

```python
def select_protocol(
    client_protocols: list[str],
    server_protocols: list[str]
) -> str:
    """选择最优协议"""
    
    # 协议优先级（从高到低）
    protocol_priority = [
        "nexa-rpc-v1",    # Nexa-net 专用协议
        "grpc",           # gRPC over HTTP/2
        "flatbuffers",    # FlatBuffers RPC
        "http2-binary",   # HTTP/2 + 二进制
    ]
    
    # 找到双方都支持的最高优先级协议
    for protocol in protocol_priority:
        if protocol in client_protocols and protocol in server_protocols:
            return protocol
    
    # 无匹配协议
    raise NegotiationError("No compatible protocol")
```

#### 2.4.2 编码选择

```python
def select_encoding(
    client_encodings: list[str],
    server_encodings: list[str],
    data_size_estimate: int
) -> str:
    """选择最优编码"""
    
    # 根据数据大小选择
    if data_size_estimate > 1_000_000:  # > 1MB
        # 大数据优先压缩率
        priority = ["gzip", "zstd", "lz4", "none"]
    else:
        # 小数据优先速度
        priority = ["lz4", "none", "zstd", "gzip"]
    
    for encoding in priority:
        if encoding in client_encodings and encoding in server_encodings:
            return encoding
    
    return "none"
```

### 2.5 Schema 传递

#### 2.5.1 Schema 压缩

```python
def compress_schema(schema: dict) -> bytes:
    """压缩 Schema 以减少传输开销"""
    
    # 1. 转换为 Protobuf 格式
    schema_proto = convert_to_proto(schema)
    
    # 2. 序列化
    serialized = schema_proto.SerializeToString()
    
    # 3. 压缩
    compressed = lz4_compress(serialized)
    
    return compressed

def decompress_schema(compressed: bytes) -> dict:
    """解压 Schema"""
    
    # 1. 解压
    decompressed = lz4_decompress(compressed)
    
    # 2. 反序列化
    schema_proto = SchemaProto.FromString(decompressed)
    
    # 3. 转换为字典
    schema = convert_from_proto(schema_proto)
    
    return schema
```

---

## 3. 结构化流式 RPC

### 3.1 RPC 模型

Nexa-net 支持三种 RPC 模型：

```
┌─────────────────────────────────────────────────────────────┐
│                    RPC Models                               │
│                                                             │
│  1. Unary RPC (一元调用)                                    │
│  ┌─────────┐         ┌─────────┐                           │
│  │ Client  │────────▶│ Server  │                           │
│  │ Request │         │ Response│                           │
│  └─────────┘         └─────────┘                           │
│  单次请求-响应，适合小数据量                                 │
│                                                             │
│  2. Server Streaming RPC (服务端流式)                       │
│  ┌─────────┐         ┌─────────┐                           │
│  │ Client  │────────▶│ Server  │                           │
│  │ Request │         │ Stream  │──▶ Response 1            │
│  └─────────┘         └─────────┘──▶ Response 2            │
│                                 ──▶ Response 3            │
│  单次请求，流式响应，适合大数据返回                          │
│                                                             │
│  3. Bidirectional Streaming RPC (双向流式)                  │
│  ┌─────────┐         ┌─────────┐                           │
│  │ Client  │──▶ Req1 │ Server  │──▶ Res1                  │
│  │ Stream  │──▶ Req2 │ Stream  │──▶ Res2                  │
│  │         │──▶ Req3 │         │──▶ Res3                  │
│  └─────────┘         └─────────┘                           │
│  双向流式，适合实时交互                                      │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### 3.2 流式传输协议

#### 3.2.1 消息帧格式

```
┌─────────────────────────────────────────────────────────────┐
│                    Message Frame Format                     │
│                                                             │
│  ┌──────────────────────────────────────────────────────┐  │
│  │                    Frame Header                       │  │
│  │  ┌────────┬────────┬────────┬────────┬─────────────┐ │  │
│  │  │ Length │ Type   │ Stream │ Flags  │ Reserved    │ │  │
│  │  │ 4 bytes│ 1 byte │ 4 bytes│ 1 byte │ 2 bytes     │ │  │
│  │  └────────┴────────┴────────┴────────┴─────────────┘ │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                             │
│  ┌──────────────────────────────────────────────────────┐  │
│  │                    Frame Payload                      │  │
│  │  ┌─────────────────────────────────────────────────┐ │  │
│  │  │                 Message Data                     │ │  │
│  │  │                 (variable length)                │ │  │
│  │  └─────────────────────────────────────────────────┘ │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                             │
│  Total: 12 bytes header + N bytes payload                  │
└─────────────────────────────────────────────────────────────┘
```

#### 3.2.2 帧类型定义

```protobuf
enum FrameType {
  // 数据帧
  DATA = 0;
  
  // 头部帧
  HEADERS = 1;
  
  // 设置优先级
  PRIORITY = 2;
  
  // 流结束
  END_STREAM = 3;
  
  // 窗口更新
  WINDOW_UPDATE = 4;
  
  // Ping
  PING = 5;
  
  // 取消流
  CANCEL = 6;
  
  // 错误
  ERROR = 7;
}
```

#### 3.2.3 标志位定义

```protobuf
enum FrameFlags {
  // 无标志
  NONE = 0;
  
  // 流结束
  END_STREAM = 0x01;
  
  // 帧结束
  END_FRAME = 0x02;
  
  // 压缩
  COMPRESSED = 0x04;
  
  // 最后一个头部块
  END_HEADERS = 0x08;
  
  // Ping 响应
  ACK = 0x10;
}
```

### 3.3 RPC 调用流程

#### 3.3.1 Unary RPC

```
┌─────────────┐                    ┌─────────────┐
│  Nexa-Proxy │                    │  Nexa-Proxy │
│     (A)     │                    │     (B)     │
└──────┬──────┘                    └──────┬──────┘
       │                                  │
       │ HEADERS Frame (Stream ID: 1)     │
       │─────────────────────────────────▶│
       │                                  │
       │ {                                │
       │   method: "translate_document",  │
       │   metadata: {...}                │
       │ }                                │
       │                                  │
       │ DATA Frame (Stream ID: 1)        │
       │─────────────────────────────────▶│
       │                                  │
       │ <compressed request payload>     │
       │                                  │
       │ END_STREAM Flag                  │
       │─────────────────────────────────▶│
       │                                  │
       │                                  │ Process request
       │                                  │
       │ HEADERS Frame (Stream ID: 1)     │
       │◀─────────────────────────────────│
       │                                  │
       │ {                                │
       │   status: "success",             │
       │   cost: 25                       │
       │ }                                │
       │                                  │
       │ DATA Frame (Stream ID: 1)        │
       │◀─────────────────────────────────│
       │                                  │
       │ <compressed response payload>    │
       │                                  │
       │ END_STREAM Flag                  │
       │◀─────────────────────────────────│
       │                                  │
```

#### 3.3.2 Server Streaming RPC

```
┌─────────────┐                    ┌─────────────┐
│  Nexa-Proxy │                    │  Nexa-Proxy │
│     (A)     │                    │     (B)     │
└──────┬──────┘                    └──────┬──────┘
       │                                  │
       │ HEADERS + DATA + END_STREAM      │
       │─────────────────────────────────▶│
       │                                  │
       │                                  │ Process
       │                                  │
       │ DATA Frame (Stream ID: 1)        │
       │◀─────────────────────────────────│
       │                                  │
       │ <chunk 1 of response>            │
       │                                  │
       │ DATA Frame (Stream ID: 1)        │
       │◀─────────────────────────────────│
       │                                  │
       │ <chunk 2 of response>            │
       │                                  │
       │ DATA Frame (Stream ID: 1)        │
       │◀─────────────────────────────────│
       │                                  │
       │ <chunk 3 of response>            │
       │                                  │
       │ END_STREAM Flag                  │
       │◀─────────────────────────────────│
       │                                  │
```

### 3.4 多路复用

#### 3.4.1 流管理

```typescript
interface StreamManager {
  // 创建新流
  createStream(): StreamId;
  
  // 获取流状态
  getStreamState(streamId: StreamId): StreamState;
  
  // 关闭流
  closeStream(streamId: StreamId): void;
  
  // 取消流
  cancelStream(streamId: StreamId, reason: string): void;
  
  // 获取所有活跃流
  getActiveStreams(): StreamId[];
}

interface StreamState {
  streamId: number;
  state: "idle" | "open" | "half_closed_local" | "half_closed_remote" | "closed";
  bytesSent: number;
  bytesReceived: number;
  createdAt: Date;
}
```

#### 3.4.2 流量控制

```python
class FlowController:
    """流量控制器，防止发送方淹没接收方"""
    
    def __init__(self, initial_window: int = 65535):
        self.window_size = initial_window
        self.bytes_sent = 0
        self.bytes_received = 0
    
    def can_send(self, size: int) -> bool:
        """检查是否可以发送"""
        return self.window_size - self.bytes_sent >= size
    
    def update_window(self, increment: int):
        """更新窗口大小"""
        self.window_size += increment
    
    def on_send(self, size: int):
        """发送后更新"""
        self.bytes_sent += size
    
    def on_receive(self, size: int):
        """接收后更新"""
        self.bytes_received += size
        # 发送 WINDOW_UPDATE 帧
        if self.bytes_received >= self.window_size // 2:
            self._send_window_update()
```

---

## 4. 二进制序列化

### 4.1 序列化格式选择

Nexa-net 支持两种主要序列化格式：

| 格式 | 优势 | 劣势 | 适用场景 |
|------|------|------|----------|
| **Protobuf** | 成熟稳定、跨语言支持好 | 需要预定义 Schema | 通用 RPC |
| **FlatBuffers** | 零拷贝、解析极快 | Schema 复杂 | 高性能场景 |

### 4.2 Protobuf Schema 定义

#### 4.2.1 基础消息类型

```protobuf
// nexa_messages.proto

syntax = "proto3";

package nexa.rpc;

// RPC 请求头
message RpcHeader {
  // 方法名
  string method = 1;
  
  // 调用 ID
  uint64 call_id = 2;
  
  // 调用方 DID
  string caller_did = 3;
  
  // 目标 endpoint ID
  string endpoint_id = 4;
  
  // 预算
  uint32 budget = 5;
  
  // 超时（毫秒）
  uint32 timeout_ms = 6;
  
  // 元数据
  map<string, string> metadata = 7;
  
  // 时间戳
  uint64 timestamp = 8;
  
  // 签名
  bytes signature = 9;
}

// RPC 响应头
message RpcResponseHeader {
  // 调用 ID（对应请求）
  uint64 call_id = 1;
  
  // 状态码
  RpcStatus status = 2;
  
  // 实际成本
  uint32 actual_cost = 3;
  
  // 处理时间（毫秒）
  uint32 processing_time_ms = 4;
  
  // 错误信息（如果失败）
  string error_message = 5;
  
  // 错误详情
  ErrorDetail error_detail = 6;
  
  // 时间戳
  uint64 timestamp = 7;
  
  // 签名
  bytes signature = 8;
}

enum RpcStatus {
  SUCCESS = 0;
  ERROR = 1;
  TIMEOUT = 2;
  CANCELLED = 3;
  INSUFFICIENT_BUDGET = 4;
  RATE_LIMITED = 5;
  INTERNAL_ERROR = 6;
}

message ErrorDetail {
  // 错误码
  string error_code = 1;
  
  // 错误类型
  ErrorType error_type = 2;
  
  // 重试建议
  RetryPolicy retry_policy = 3;
}

enum ErrorType {
  TRANSIENT = 0;    // 临时错误，可重试
  PERMANENT = 1;    // 永久错误，不可重试
  CLIENT = 2;       // 客户端错误
  SERVER = 3;       // 服务端错误
}

message RetryPolicy {
  // 是否可重试
  bool retryable = 1;
  
  // 最大重试次数
  uint32 max_retries = 2;
  
  // 初始延迟（毫秒）
  uint32 initial_delay_ms = 3;
  
  // 最大延迟（毫秒）
  uint32 max_delay_ms = 4;
  
  // 延迟乘数
  float delay_multiplier = 5;
}
```

#### 4.2.2 数据消息

```protobuf
// 数据帧
message DataFrame {
  // 流 ID
  uint32 stream_id = 1;
  
  // 序列号
  uint64 sequence = 2;
  
  // 是否压缩
  bool compressed = 3;
  
  // 数据
  bytes data = 4;
  
  // 数据类型
  DataType data_type = 5;
}

enum DataType {
  // 二进制数据
  BINARY = 0;
  
  // 文本数据
  TEXT = 1;
  
  // JSON 数据
  JSON = 2;
  
  // Protobuf 数据
  PROTOBUF = 3;
  
  // 图像数据
  IMAGE = 4;
  
  // 音频数据
  AUDIO = 5;
  
  // 视频数据
  VIDEO = 6;
}
```

### 4.3 FlatBuffers Schema 定义

```flatbuffers
// nexa_messages.fbs

namespace nexa.rpc;

table RpcHeader {
  method: string;
  call_id: ulong;
  caller_did: string;
  endpoint_id: string;
  budget: uint;
  timeout_ms: uint;
  metadata: MetadataEntry[];
  timestamp: ulong;
  signature: [ubyte];
}

table MetadataEntry {
  key: string;
  value: string;
}

table RpcResponseHeader {
  call_id: ulong;
  status: RpcStatus;
  actual_cost: uint;
  processing_time_ms: uint;
  error_message: string;
  error_detail: ErrorDetail;
  timestamp: ulong;
  signature: [ubyte];
}

enum RpcStatus: byte {
  SUCCESS,
  ERROR,
  TIMEOUT,
  CANCELLED,
  INSUFFICIENT_BUDGET,
  RATE_LIMITED,
  INTERNAL_ERROR
}

table ErrorDetail {
  error_code: string;
  error_type: ErrorType;
  retry_policy: RetryPolicy;
}

enum ErrorType: byte {
  TRANSIENT,
  PERMANENT,
  CLIENT,
  SERVER
}

table RetryPolicy {
  retryable: bool;
  max_retries: uint;
  initial_delay_ms: uint;
  max_delay_ms: uint;
  delay_multiplier: float;
}

table DataFrame {
  stream_id: uint;
  sequence: ulong;
  compressed: bool;
  data: [ubyte];
  data_type: DataType;
}

enum DataType: byte {
  BINARY,
  TEXT,
  JSON,
  PROTOBUF,
  IMAGE,
  AUDIO,
  VIDEO
}
```

### 4.4 序列化性能对比

```
┌─────────────────────────────────────────────────────────────┐
│              Serialization Performance Comparison           │
│                                                             │
│  测试数据：100KB JSON 文档                                   │
│                                                             │
│  ┌──────────────────────────────────────────────────────┐  │
│  │                    Serialization Time                 │  │
│  │                                                      │  │
│  │  JSON:     ████████████████████████████████  15ms    │  │
│  │  Protobuf: ████                                2ms   │  │
│  │  FlatBuf:  ██                                 1ms    │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                             │
│  ┌──────────────────────────────────────────────────────┐  │
│  │                    Serialized Size                    │  │
│  │                                                      │  │
│  │  JSON:     ████████████████████████████████  100KB   │  │
│  │  Protobuf: ████████                          35KB    │  │
│  │  FlatBuf:  ██████                            28KB    │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                             │
│  ┌──────────────────────────────────────────────────────┐  │
│  │                    Deserialization Time               │  │
│  │                                                      │  │
│  │  JSON:     ████████████████████████████████  20ms    │  │
│  │  Protobuf: ██████                            3ms     │  │
│  │  FlatBuf:  █                                 0.5ms   │  │ (零拷贝)
│  └──────────────────────────────────────────────────────┘  │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## 5. 连接管理

### 5.1 连接池

```typescript
interface ConnectionPool {
  // 获取连接
  acquire(targetDID: string): Promise<Connection>;
  
  // 释放连接
  release(connection: Connection): void;
  
  // 预创建连接
  preconnect(targetDIDs: string[]): Promise<void>;
  
  // 关闭所有连接
  closeAll(): Promise<void>;
  
  // 获取连接状态
  getStatus(): PoolStatus;
}

interface PoolStatus {
  totalConnections: number;
  activeConnections: number;
  idleConnections: number;
  pendingRequests: number;
}

interface Connection {
  // 连接 ID
  id: string;
  
  // 目标 DID
  targetDID: string;
  
  // 连接状态
  state: "connecting" | "ready" | "busy" | "closing" | "closed";
  
  // 创建时间
  createdAt: Date;
  
  // 最后活动时间
  lastActivityAt: Date;
  
  // 活跃流数
  activeStreams: number;
  
  // 发送字节
  bytesSent: number;
  
  // 接收字节
  bytesReceived: number;
}
```

### 5.2 连接生命周期

```
┌─────────────────────────────────────────────────────────────┐
│                    Connection Lifecycle                     │
│                                                             │
│  ┌─────────┐     ┌─────────┐     ┌─────────┐     ┌─────────┐│
│  │ IDLE    │────▶│CONNECTING│────▶│ READY   │────▶│ BUSY    ││
│  │         │     │         │     │         │     │         ││
│  │ 空闲    │     │ 建立中  │     │ 就绪    │     │ 使用中  ││
│  └─────────┘     └─────────┘     └─────────┘     └─────────┘│
│       ▲                              │               │     │
│       │                              │               │     │
│       │                              ▼               ▼     │
│       │                        ┌─────────┐     ┌─────────┐ │
│       │                        │ CLOSING │────▶│ CLOSED  │ │
│       │                        │         │     │         │ │
│       │                        │ 关闭中  │     │ 已关闭  │ │
│       │                        └─────────┘     └─────────┘ │
│       │                              │                     │
│       └──────────────────────────────┘                     │
│                    (超时/错误/主动关闭)                      │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### 5.3 心跳保活

```python
class HeartbeatManager:
    """心跳管理器"""
    
    def __init__(
        self,
        interval: int = 30,    # 心跳间隔（秒）
        timeout: int = 90      # 超时时间（秒）
    ):
        self.interval = interval
        self.timeout = timeout
        self.last_heartbeat = {}
    
    async def start_heartbeat(self, connection: Connection):
        """启动心跳"""
        while connection.state == "ready":
            try:
                # 发送 Ping 帧
                ping_frame = self._create_ping_frame()
                await connection.send(ping_frame)
                
                # 等待响应
                response = await connection.receive(timeout=self.interval)
                
                if response.type == FrameType.PING and response.flags & FrameFlags.ACK:
                    self.last_heartbeat[connection.id] = datetime.utcnow()
                
            except TimeoutError:
                # 心跳超时
                age = (datetime.utcnow() - self.last_heartbeat.get(connection.id)).total_seconds()
                if age > self.timeout:
                    await connection.close(reason="heartbeat_timeout")
                    break
            
            await asyncio.sleep(self.interval)
    
    def _create_ping_frame(self) -> Frame:
        """创建 Ping 帧"""
        return Frame(
            type=FrameType.PING,
            stream_id=0,
            flags=FrameFlags.NONE,
            payload=str(uuid.uuid4()).encode()
        )
```

### 5.4 连接配置

```yaml
# connection_config.yaml
connection:
  pool:
    max_connections: 100
    max_connections_per_target: 5
    idle_timeout: 300  # 5 minutes
    acquire_timeout: 10  # 10 seconds
    
  heartbeat:
    interval: 30  # 30 seconds
    timeout: 90   # 90 seconds
    
  stream:
    max_concurrent_streams: 100
    initial_window_size: 65535
    max_frame_size: 16384
    
  timeout:
    connect: 5000   # 5 seconds
    request: 30000  # 30 seconds
    stream: 60000   # 60 seconds
```

---

## 6. 错误处理与重试

### 6.1 错误分类

```typescript
enum ErrorCategory {
  // 网络错误
  NETWORK = "network",
  
  // 协议错误
  PROTOCOL = "protocol",
  
  // 服务错误
  SERVICE = "service",
  
  // 业务错误
  BUSINESS = "business",
  
  // 资源错误
  RESOURCE = "resource",
  
  // 安全错误
  SECURITY = "security"
}

interface NexaError {
  // 错误码
  code: string;
  
  // 错误类别
  category: ErrorCategory;
  
  // 错误类型
  type: ErrorType;
  
  // 错误消息
  message: string;
  
  // 错误详情
  details: Record<string, any>;
  
  // 重试策略
  retryPolicy: RetryPolicy;
  
  // 时间戳
  timestamp: Date;
}
```

### 6.2 错误码定义

| 错误码 | 类别 | 描述 | 重试策略 |
|--------|------|------|----------|
| `TR001` | NETWORK | 连接超时 | 指数退避，最多 3 次 |
| `TR002` | NETWORK | 连接被拒绝 | 不重试 |
| `TR003` | NETWORK | DNS 解析失败 | 不重试 |
| `TR004` | PROTOCOL | 协议协商失败 | 不重试 |
| `TR005` | PROTOCOL | Schema 不兼容 | 不重试 |
| `TR006` | PROTOCOL | 序列化错误 | 不重试 |
| `TR007` | SERVICE | 服务不可用 | 指数退避，最多 5 次 |
| `TR008` | SERVICE | 服务过载 | 指数退避，最多 3 次 |
| `TR009` | SERVICE | 内部错误 | 指数退避，最多 2 次 |
| `TR010` | BUSINESS | 参数无效 | 不重试 |
| `TR011` | BUSINESS | 权限不足 | 不重试 |
| `TR012` | BUSINESS | 预算不足 | 不重试 |
| `TR013` | RESOURCE | 内存不足 | 不重试 |
| `TR014` | RESOURCE | 队列满 | 指数退避，最多 3 次 |
| `TR015` | SECURITY | 认证失败 | 不重试 |
| `TR016` | SECURITY | 签名无效 | 不重试 |

### 6.3 重试策略

#### 6.3.1 指数退避

```python
class ExponentialBackoff:
    """指数退避重试策略"""
    
    def __init__(
        self,
        initial_delay: float = 1.0,    # 初始延迟（秒）
        max_delay: float = 30.0,       # 最大延迟（秒）
        multiplier: float = 2.0,       # 延迟乘数
        max_retries: int = 3           # 最大重试次数
    ):
        self.initial_delay = initial_delay
        self.max_delay = max_delay
        self.multiplier = multiplier
        self.max_retries = max_retries
    
    def get_delay(self, retry_count: int) -> float:
        """计算第 N 次重试的延迟"""
        delay = self.initial_delay * (self.multiplier ** retry_count)
        return min(delay, self.max_delay)
    
    async def execute_with_retry(
        self,
        operation: Callable,
        is_retryable: Callable[[Exception], bool]
    ) -> Any:
        """带重试的执行"""
        last_error = None
        
        for retry in range(self.max_retries + 1):
            try:
                return await operation()
            except Exception as e:
                last_error = e
                
                if not is_retryable(e):
                    raise
                
                if retry < self.max_retries:
                    delay = self.get_delay(retry)
                    await asyncio.sleep(delay)
        
        raise last_error
```

#### 6.3.2 重试决策

```python
def is_retryable_error(error: NexaError) -> bool:
    """判断错误是否可重试"""
    
    # 临时错误可重试
    if error.type == ErrorType.TRANSIENT:
        return True
    
    # 网络错误部分可重试
    if error.category == ErrorCategory.NETWORK:
        retryable_codes = ["TR001"]  # 连接超时
        return error.code in retryable_codes
    
    # 服务错误部分可重试
    if error.category == ErrorCategory.SERVICE:
        retryable_codes = ["TR007", "TR008"]  # 服务不可用、过载
        return error.code in retryable_codes
    
    # 其他错误不重试
    return False
```

### 6.4 超时管理

```typescript
interface TimeoutManager {
  // 设置超时
  setTimeout(callId: string, timeoutMs: number): void;
  
  // 取消超时
  cancelTimeout(callId: string): void;
  
  // 检查超时
  checkTimeout(callId: string): boolean;
  
  // 清理过期超时
  cleanupExpired(): void;
}

// 超时配置
interface TimeoutConfig {
  // 连接超时
  connectTimeout: number;  // 5000ms
  
  // 请求超时
  requestTimeout: number;  // 30000ms
  
  // 流超时
  streamTimeout: number;   // 60000ms
  
  // 心跳超时
  heartbeatTimeout: number; // 90000ms
  
  // 空闲超时
  idleTimeout: number;     // 300000ms
}
```

---

## 7. 实现规范

### 7.1 接口定义

```typescript
interface TransportLayerAPI {
  // 协议协商
  negotiation: {
    initiate(targetDID: string, intent: Intent): Promise<NegotiationResult>;
    accept(negotiationRequest: SynNexa): Promise<AckSchema>;
    cancel(negotiationId: string): Promise<void>;
  };
  
  // RPC 调用
  rpc: {
    unary(targetDID: string, method: string, request: bytes): Promise<RpcResponse>;
    serverStream(targetDID: string, method: string, request: bytes): Promise<Stream>;
    bidirectionalStream(targetDID: string, method: string): Promise<BidirectionalStream>;
  };
  
  // 序列化
  serialization: {
    encode(message: any, schema: Schema): Promise<bytes>;
    decode(data: bytes, schema: Schema): Promise<any>;
    getSchema(schemaHash: string): Promise<Schema>;
  };
  
  // 连接管理
  connection: {
    acquire(targetDID: string): Promise<Connection>;
    release(connection: Connection): void;
    getStatus(): Promise<PoolStatus>;
  };
  
  // 错误处理
  error: {
    handleError(error: NexaError): Promise<ErrorAction>;
    retry(operation: Callable, policy: RetryPolicy): Promise<any>;
  };
}
```

### 7.2 配置规范

```yaml
# transport_config.yaml
transport:
  protocol:
    default: "nexa-rpc-v1"
    supported: ["nexa-rpc-v1", "grpc", "flatbuffers"]
    
  encoding:
    default: "protobuf"
    supported: ["protobuf", "flatbuffers", "json"]
    
  compression:
    default: "lz4"
    threshold: 1024  # 大于 1KB 才压缩
    
  frame:
    max_size: 16384  # 16KB
    header_size: 12
    
  stream:
    max_concurrent: 100
    initial_window: 65535
    
  connection:
    pool_size: 100
    idle_timeout: 300
    heartbeat_interval: 30
    
  timeout:
    connect: 5000
    request: 30000
    stream: 60000
    
  retry:
    max_retries: 3
    initial_delay: 1000
    max_delay: 30000
    multiplier: 2
```

---

## 8. 性能考量

### 8.1 性能目标

| 指标 | 目标值 | 测量方法 |
|------|--------|----------|
| **协议协商延迟** | < 50ms | 从 SYN-NEXA 到 ACCEPT |
| **序列化延迟** | < 2ms | 100KB 数据编码 |
| **反序列化延迟** | < 3ms | 100KB 数据解码 |
| **单次 RPC 延迟** | < 100ms | 小数据 Unary RPC |
| **流式吞吐量** | > 100MB/s | 大数据流式传输 |
| **并发流数** | > 100 | 单连接并发流 |

### 8.2 优化策略

#### 8.2.1 Schema 缓存

```python
class SchemaCache:
    """Schema 缓存，避免重复传输"""
    
    def __init__(self, max_size: int = 1000):
        self.cache = {}
        self.max_size = max_size
    
    def get(self, schema_hash: str) -> Optional[Schema]:
        return self.cache.get(schema_hash)
    
    def put(self, schema_hash: str, schema: Schema):
        if len(self.cache) >= self.max_size:
            self._evict_oldest()
        self.cache[schema_hash] = schema
    
    def has(self, schema_hash: str) -> bool:
        return schema_hash in self.cache
```

#### 8.2.2 连接预热

```python
async def preconnect_targets(
    discovery_result: RoutingResult,
    pool: ConnectionPool
):
    """根据路由结果预热连接"""
    
    # 获取前 3 个候选
    top_candidates = discovery_result.candidates[:3]
    
    # 预创建连接
    for candidate in top_candidates:
        try:
            await pool.preconnect(candidate.did)
        except Exception:
            # 预连接失败不影响主流程
            pass
```

#### 8.2.3 批量序列化

```python
async def batch_encode(
    messages: list[any],
    schema: Schema,
    serializer: Serializer
) -> list[bytes]:
    """批量序列化，利用 SIMD 优化"""
    
    # 并行序列化
    tasks = [serializer.encode(msg, schema) for msg in messages]
    results = await asyncio.gather(*tasks)
    
    return results
```

---

## 9. 相关文档

### 上层架构

- [整体架构设计](./ARCHITECTURE.md) - 四层架构总览
- [语义发现与能力路由层](./DISCOVERY_LAYER.md) - Layer 2 设计

### 下层依赖

- [资源管理与微交易层](./ECONOMY_LAYER.md) - Layer 4 设计

### 相关规范

- [协议规范](./PROTOCOL_SPEC.md) - 传输层协议定义
- [API 参考](./API_REFERENCE.md) - 传输层 API 定义
- [安全设计](./SECURITY.md) - 传输层安全机制

### 参考资料

- [Protocol Buffers](https://protobuf.dev/)
- [FlatBuffers](https://flatbuffers.dev/)
- [gRPC Specification](https://grpc.io/docs/)
- [HTTP/2 Specification](https://httpwg.org/specs/rfc7540.html)
- [QUIC Protocol](https://www.quic.org/)