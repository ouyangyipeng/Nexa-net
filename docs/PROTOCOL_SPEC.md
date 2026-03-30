# Nexa-net 协议规范

> **版本:** v1.0.0-draft | **最后更新:** 2026-03-30

## 目录

- [1. 协议概述](#1-协议概述)
- [2. 消息格式](#2-消息格式)
- [3. 身份协议](#3-身份协议)
- [4. 发现协议](#4-发现协议)
- [5. 传输协议](#5-传输协议)
- [6. 经济协议](#6-经济协议)
- [7. 错误处理](#7-错误处理)
- [8. 版本协商](#8-版本协商)
- [9. 相关文档](#9-相关文档)

---

## 1. 协议概述

### 1.1 协议栈

Nexa-net 协议栈分为四层：

```
┌─────────────────────────────────────────────────────────────┐
│                    Nexa-net Protocol Stack                  │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Layer 4: Economy Protocol              │   │
│  │  - 通道管理协议 (Channel Management)                 │   │
│  │  - 收据协议 (Receipt Protocol)                       │   │
│  │  - 结算协议 (Settlement Protocol)                    │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Layer 3: Transport Protocol            │   │
│  │  - RPC 协议 (RPC Protocol)                           │   │
│  │  - 流式传输协议 (Streaming Protocol)                  │   │
│  │  - 协商协议 (Negotiation Protocol)                   │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Layer 2: Discovery Protocol            │   │
│  │  - 注册协议 (Registration Protocol)                  │   │
│  │  - 路由协议 (Routing Protocol)                       │   │
│  │  - 心跳协议 (Heartbeat Protocol)                     │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Layer 1: Identity Protocol             │   │
│  │  - DID 协议 (DID Protocol)                           │   │
│  │  - 认证协议 (Authentication Protocol)                │   │
│  │  - 凭证协议 (Credential Protocol)                    │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Transport Layer                        │   │
│  │  - TLS 1.3 / QUIC                                   │   │
│  │  - HTTP/2 / HTTP/3                                  │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### 1.2 协议版本

| 版本 | 状态 | 发布日期 | 描述 |
|------|------|----------|------|
| `v1.0.0` | 草稿 | 2026-03-30 | 初始版本 |

### 1.3 协议标识

```
nexa://<version>/<protocol>/<message_type>

示例：
nexa://v1/identity/did_resolve
nexa://v1/discovery/register
nexa://v1/transport/rpc_call
nexa://v1/economy/channel_open
```

---

## 2. 消息格式

### 2.1 消息结构

所有 Nexa-net 消息遵循统一的结构：

```protobuf
// nexa_message.proto

syntax = "proto3";

package nexa.protocol;

import "google/protobuf/timestamp.proto";
import "google/protobuf/any.proto";

// Nexa-net 消息封装
message NexaMessage {
  // 消息头
  MessageHeader header = 1;
  
  // 消息体
  google.protobuf.Any body = 2;
  
  // 消息签名
  MessageSignature signature = 3;
}

// 消息头
message MessageHeader {
  // 协议版本
  string protocol_version = 1;
  
  // 消息类型
  string message_type = 2;
  
  // 消息 ID（用于追踪和去重）
  string message_id = 3;
  
  // 关联消息 ID（请求-响应关联）
  string correlation_id = 4;
  
  // 发送方 DID
  string sender_did = 5;
  
  // 接收方 DID
  string receiver_did = 6;
  
  // 时间戳
  google.protobuf.Timestamp timestamp = 7;
  
  // TTL（秒）
  uint32 ttl = 8;
  
  // 元数据
  map<string, string> metadata = 9;
}

// 消息签名
message MessageSignature {
  // 签名算法
  string algorithm = 1;  // Ed25519, Secp256k1
  
  // 签名值
  bytes signature = 2;
  
  // 签名时间戳
  google.protobuf.Timestamp signed_at = 3;
  
  // 验证公钥引用
  string verification_key_ref = 4;
}
```

### 2.2 消息类型定义

```protobuf
// message_types.proto

syntax = "proto3";

package nexa.protocol;

// 消息类型枚举
enum MessageType {
  // 未知类型
  MESSAGE_TYPE_UNSPECIFIED = 0;
  
  // ===== Layer 1: Identity =====
  // DID 相关
  MESSAGE_TYPE_DID_GENERATE = 100;
  MESSAGE_TYPE_DID_RESOLVE = 101;
  MESSAGE_TYPE_DID_RESOLVE_RESPONSE = 102;
  MESSAGE_TYPE_DID_UPDATE = 103;
  MESSAGE_TYPE_DID_DEACTIVATE = 104;
  
  // 认证相关
  MESSAGE_TYPE_AUTH_CHALLENGE = 110;
  MESSAGE_TYPE_AUTH_RESPONSE = 111;
  MESSAGE_TYPE_AUTH_VERIFY = 112;
  
  // 凭证相关
  MESSAGE_TYPE_VC_ISSUE = 120;
  MESSAGE_TYPE_VC_VERIFY = 121;
  MESSAGE_TYPE_VC_PRESENT = 122;
  
  // ===== Layer 2: Discovery =====
  // 注册相关
  MESSAGE_TYPE_REGISTER = 200;
  MESSAGE_TYPE_REGISTER_RESPONSE = 201;
  MESSAGE_TYPE_UNREGISTER = 202;
  MESSAGE_TYPE_HEARTBEAT = 203;
  
  // 路由相关
  MESSAGE_TYPE_ROUTE_QUERY = 210;
  MESSAGE_TYPE_ROUTE_RESPONSE = 211;
  MESSAGE_TYPE_CAPABILITY_QUERY = 212;
  MESSAGE_TYPE_CAPABILITY_RESPONSE = 213;
  
  // ===== Layer 3: Transport =====
  // 协商相关
  MESSAGE_TYPE_SYN_NEXA = 300;
  MESSAGE_TYPE_ACK_SCHEMA = 301;
  MESSAGE_TYPE_ACCEPT = 302;
  MESSAGE_TYPE_REJECT = 303;
  
  // RPC 相关
  MESSAGE_TYPE_RPC_REQUEST = 310;
  MESSAGE_TYPE_RPC_RESPONSE = 311;
  MESSAGE_TYPE_RPC_STREAM = 312;
  MESSAGE_TYPE_RPC_CANCEL = 313;
  
  // ===== Layer 4: Economy =====
  // 通道相关
  MESSAGE_TYPE_CHANNEL_OPEN = 400;
  MESSAGE_TYPE_CHANNEL_OPEN_RESPONSE = 401;
  MESSAGE_TYPE_CHANNEL_CLOSE = 402;
  MESSAGE_TYPE_CHANNEL_CLOSE_RESPONSE = 403;
  
  // 收据相关
  MESSAGE_TYPE_RECEIPT_CREATE = 410;
  MESSAGE_TYPE_RECEIPT_SIGN = 411;
  MESSAGE_TYPE_RECEIPT_VERIFY = 412;
  
  // 结算相关
  MESSAGE_TYPE_SETTLEMENT_REQUEST = 420;
  MESSAGE_TYPE_SETTLEMENT_RESPONSE = 421;
  MESSAGE_TYPE_SETTLEMENT_DISPUTE = 422;
}
```

### 2.3 消息序列化

消息支持多种序列化格式：

| 格式 | 内容类型 | 适用场景 |
|------|----------|----------|
| **Protobuf** | `application/x-protobuf` | 默认，高性能 |
| **FlatBuffers** | `application/x-flatbuffers` | 零拷贝场景 |
| **JSON** | `application/json` | 调试、兼容 |

---

## 3. 身份协议

### 3.1 DID 协议

#### 3.1.1 DID 生成

```protobuf
// identity.proto

message DidGenerateRequest {
  // 密钥算法
  string key_algorithm = 1;  // Ed25519, Secp256k1
  
  // 是否生成加密密钥对
  bool generate_encryption_key = 2;
  
  // 元数据
  map<string, string> metadata = 3;
}

message DidGenerateResponse {
  // 生成的 DID
  string did = 1;
  
  // DID Document
  DidDocument document = 2;
  
  // 私钥（加密后）
  bytes encrypted_private_key = 3;
  
  // 公钥
  bytes public_key = 4;
  
  // 加密公钥（如果生成）
  bytes encryption_public_key = 5;
}

message DidDocument {
  // @context
  repeated string context = 1;
  
  // DID
  string id = 2;
  
  // 控制者
  string controller = 3;
  
  // 验证方法
  repeated VerificationMethod verification_method = 4;
  
  // 认证方法引用
  repeated string authentication = 5;
  
  // 密钥协商方法
  repeated VerificationMethod key_agreement = 6;
  
  // 服务端点
  repeated Service service = 7;
  
  // 创建时间
  google.protobuf.Timestamp created = 8;
  
  // 更新时间
  google.protobuf.Timestamp updated = 9;
}

message VerificationMethod {
  string id = 1;
  string type = 2;
  string controller = 3;
  bytes public_key_multibase = 4;
}

message Service {
  string id = 1;
  string type = 2;
  string service_endpoint = 3;
}
```

#### 3.1.2 DID 解析

```protobuf
message DidResolveRequest {
  // 要解析的 DID
  string did = 1;
  
  // 解析选项
  ResolveOptions options = 2;
}

message ResolveOptions {
  // 是否返回元数据
  bool include_metadata = 1;
  
  // 是否验证签名
  bool verify_signature = 2;
  
  // 缓存控制
  CacheControl cache_control = 3;
}

enum CacheControl {
  CACHE_CONTROL_UNSPECIFIED = 0;
  CACHE_CONTROL_CACHE_FIRST = 1;
  CACHE_CONTROL_NETWORK_FIRST = 2;
  CACHE_CONTROL_CACHE_ONLY = 3;
  CACHE_CONTROL_NETWORK_ONLY = 4;
}

message DidResolveResponse {
  // DID Document
  DidDocument document = 1;
  
  // 解析元数据
  DidResolutionMetadata metadata = 2;
}

message DidResolutionMetadata {
  // 内容类型
  string content_type = 1;
  
  // 是否已缓存
  bool cached = 2;
  
  // 缓存过期时间
  google.protobuf.Timestamp cache_expires = 3;
  
  // 解析时间
  google.protobuf.Timestamp resolved_at = 4;
}
```

### 3.2 认证协议

#### 3.2.1 挑战-响应认证

```protobuf
message AuthChallenge {
  // 挑战 ID
  string challenge_id = 1;
  
  // 挑战数据（随机数）
  bytes challenge_data = 2;
  
  // 过期时间
  google.protobuf.Timestamp expires_at = 3;
  
  // 支持的签名算法
  repeated string supported_algorithms = 4;
}

message AuthResponse {
  // 挑战 ID
  string challenge_id = 1;
  
  // 签名者 DID
  string signer_did = 2;
  
  // 签名算法
  string algorithm = 3;
  
  // 签名值
  bytes signature = 4;
  
  // 时间戳
  google.protobuf.Timestamp timestamp = 5;
}

message AuthVerifyRequest {
  // 挑战 ID
  string challenge_id = 1;
  
  // 认证响应
  AuthResponse response = 2;
}

message AuthVerifyResponse {
  // 验证结果
  bool valid = 1;
  
  // 签名者 DID
  string signer_did = 2;
  
  // 错误信息（如果失败）
  string error_message = 3;
}
```

### 3.3 凭证协议

```protobuf
message VcIssueRequest {
  // 签发者 DID
  string issuer_did = 1;
  
  // 主体 DID
  string subject_did = 2;
  
  // 凭证类型
  repeated string types = 3;
  
  // 凭证声明
  map<string, google.protobuf.Any> claims = 4;
  
  // 有效期（秒）
  uint32 validity_seconds = 5;
}

message VcIssueResponse {
  // 可验证凭证
  VerifiableCredential credential = 1;
}

message VerifiableCredential {
  // @context
  repeated string context = 1;
  
  // ID
  string id = 2;
  
  // 类型
  repeated string type = 3;
  
  // 签发者
  Issuer issuer = 4;
  
  // 签发日期
  google.protobuf.Timestamp issuance_date = 5;
  
  // 过期日期
  google.protobuf.Timestamp expiration_date = 6;
  
  // 凭证主体
  CredentialSubject credential_subject = 7;
  
  // 证明
  Proof proof = 8;
}

message Issuer {
  string id = 1;
  string name = 2;
}

message CredentialSubject {
  string id = 1;
  map<string, google.protobuf.Any> claims = 2;
}

message Proof {
  string type = 1;
  google.protobuf.Timestamp created = 2;
  string verification_method = 3;
  string proof_purpose = 4;
  bytes proof_value = 5;
}

message VcVerifyRequest {
  // 可验证凭证
  VerifiableCredential credential = 1;
  
  // 验证选项
  VerifyOptions options = 2;
}

message VerifyOptions {
  // 是否验证过期
  bool check_expiration = 1;
  
  // 是否验证签发者信任
  bool check_issuer_trust = 2;
  
  // 是否验证撤销状态
  bool check_revocation = 3;
}

message VcVerifyResponse {
  // 验证结果
  bool valid = 1;
  
  // 验证详情
  repeated VerificationCheck checks = 2;
  
  // 错误信息
  string error_message = 3;
}

message VerificationCheck {
  string check_type = 1;
  bool passed = 2;
  string message = 3;
}
```

---

## 4. 发现协议

### 4.1 注册协议

```protobuf
// discovery.proto

message RegisterRequest {
  // 注册者 DID
  string did = 1;
  
  // 能力清单
  CapabilitySchema capability_schema = 2;
  
  // 端点信息
  EndpointInfo endpoint_info = 3;
  
  // 签名
  bytes signature = 4;
}

message CapabilitySchema {
  // 版本
  string version = 1;
  
  // 元数据
  CapabilityMetadata metadata = 2;
  
  // 端点列表
  repeated EndpointSchema endpoints = 3;
  
  // 预计算向量
  repeated VectorEntry semantic_vectors = 4;
}

message CapabilityMetadata {
  string name = 1;
  string description = 2;
  repeated string tags = 3;
}

message EndpointSchema {
  string id = 1;
  string name = 2;
  string description = 3;
  google.protobuf.Struct input_schema = 4;
  google.protobuf.Struct output_schema = 5;
  CostModel cost = 6;
  RateLimit rate_limit = 7;
  QualityMetrics quality = 8;
}

message CostModel {
  string model = 1;  // per_call, per_page, per_token, per_byte
  uint64 base_price = 2;
  repeated CostModifier modifiers = 3;
}

message CostModifier {
  string condition = 1;
  float multiplier = 2;
}

message RateLimit {
  uint32 max_concurrent = 1;
  uint32 max_per_minute = 2;
  uint32 max_per_day = 3;
}

message QualityMetrics {
  float accuracy_score = 1;
  uint32 avg_latency_ms = 2;
  float availability = 3;
}

message VectorEntry {
  string endpoint_id = 1;
  repeated float vector = 2;
}

message EndpointInfo {
  string address = 1;
  uint32 port = 2;
  string protocol = 3;
  map<string, string> metadata = 4;
}

message RegisterResponse {
  // 注册 ID
  string registration_id = 1;
  
  // 状态
  RegistrationStatus status = 2;
  
  // 过期时间
  google.protobuf.Timestamp expires_at = 3;
  
  // 错误信息
  string error_message = 4;
}

enum RegistrationStatus {
  REGISTRATION_STATUS_UNSPECIFIED = 0;
  REGISTRATION_STATUS_SUCCESS = 1;
  REGISTRATION_STATUS_PENDING = 2;
  REGISTRATION_STATUS_FAILED = 3;
}
```

### 4.2 路由协议

```protobuf
message RouteQuery {
  // 意图描述
  string intent = 1;
  
  // 意图向量（可选，如果提供则跳过向量化）
  repeated float intent_vector = 2;
  
  // 查询参数
  RouteQueryParams params = 3;
}

message RouteQueryParams {
  // 返回结果数量
  uint32 top_k = 1;
  
  // 相似度阈值
  float similarity_threshold = 2;
  
  // 过滤条件
  RouteFilter filter = 3;
}

message RouteFilter {
  // 最大成本
  uint64 max_cost = 1;
  
  // 最低质量分数
  float min_quality_score = 2;
  
  // 最大延迟
  uint32 max_latency_ms = 3;
  
  // 排除的 DID
  repeated string excluded_dids = 4;
  
  // 首选的 DID
  repeated string preferred_dids = 5;
}

message RouteResponse {
  // 候选列表
  repeated RouteCandidate candidates = 1;
  
  // 查询元数据
  RouteMetadata metadata = 2;
}

message RouteCandidate {
  // 排名
  uint32 rank = 1;
  
  // 服务提供者 DID
  string provider_did = 2;
  
  // 端点 ID
  string endpoint_id = 3;
  
  // 相似度分数
  float similarity = 4;
  
  // 预估成本
  uint64 estimated_cost = 5;
  
  // 预估延迟
  uint32 estimated_latency_ms = 6;
  
  // 质量分数
  float quality_score = 7;
  
  // 元数据
  map<string, string> metadata = 8;
}

message RouteMetadata {
  // 查询耗时
  uint32 query_time_ms = 1;
  
  // 总候选数
  uint32 total_candidates = 2;
  
  // 过滤后数量
  uint32 filtered_count = 3;
}
```

### 4.3 心跳协议

```protobuf
message Heartbeat {
  // 发送者 DID
  string sender_did = 1;
  
  // 时间戳
  google.protobuf.Timestamp timestamp = 2;
  
  // 节点状态
  NodeStatus status = 3;
  
  // 序列号
  uint64 sequence = 4;
}

message NodeStatus {
  // 在线状态
  bool online = 1;
  
  // 负载
  NodeLoad load = 2;
  
  // 性能指标
  NodePerformance performance = 3;
}

message NodeLoad {
  float cpu = 1;
  float memory = 2;
  uint32 concurrent_calls = 3;
  uint32 queue_length = 4;
}

message NodePerformance {
  uint32 avg_latency_ms = 1;
  uint32 p99_latency_ms = 2;
  float success_rate = 3;
  float throughput = 4;
}

message HeartbeatAck {
  // 收到的时间戳
  google.protobuf.Timestamp received_at = 1;
  
  // 服务器时间
  google.protobuf.Timestamp server_time = 2;
}
```

---

## 5. 传输协议

### 5.1 协商协议

```protobuf
// transport.proto

message SynNexa {
  // 意图哈希
  string intent_hash = 1;
  
  // 最大预算
  uint64 max_budget = 2;
  
  // 支持的协议
  repeated string supported_protocols = 3;
  
  // 支持的编码
  repeated string supported_encodings = 4;
  
  // 支持的压缩算法
  repeated CompressionType supported_compressions = 5;
  
  // 客户端能力
  ClientCapabilities capabilities = 6;
  
  // 时间戳
  google.protobuf.Timestamp timestamp = 7;
  
  // 签名
  bytes signature = 8;
}

enum CompressionType {
  COMPRESSION_TYPE_UNSPECIFIED = 0;
  COMPRESSION_TYPE_NONE = 1;
  COMPRESSION_TYPE_GZIP = 2;
  COMPRESSION_TYPE_LZ4 = 3;
  COMPRESSION_TYPE_ZSTD = 4;
}

message ClientCapabilities {
  uint32 max_concurrent_streams = 1;
  uint64 max_message_size = 2;
  bool streaming = 3;
  bool bidirectional = 4;
}

message AckSchema {
  // 选定的协议
  string selected_protocol = 1;
  
  // 选定的编码
  string selected_encoding = 2;
  
  // 选定的压缩算法
  CompressionType selected_compression = 3;
  
  // Schema 哈希
  string schema_hash = 4;
  
  // 压缩后的 Schema
  bytes compressed_schema = 5;
  
  // 预估成本
  uint64 estimated_cost = 6;
  
  // 预估延迟
  uint32 estimated_latency_ms = 7;
  
  // 服务端能力
  ServerCapabilities capabilities = 8;
  
  // 时间戳
  google.protobuf.Timestamp timestamp = 9;
  
  // 签名
  bytes signature = 10;
}

message ServerCapabilities {
  uint32 max_concurrent_streams = 1;
  uint64 max_message_size = 2;
  float current_load = 3;
  uint32 available_queue_slots = 4;
}

message Accept {
  // 会话 ID
  string session_id = 1;
  
  // 是否就绪
  bool ready = 2;
  
  // 错误信息（如果不就绪）
  string error_message = 3;
}

message Reject {
  // 拒绝原因
  RejectReason reason = 1;
  
  // 详细信息
  string message = 2;
  
  // 建议的替代方案
  repeated string alternatives = 3;
}

enum RejectReason {
  REJECT_REASON_UNSPECIFIED = 0;
  REJECT_REASON_UNSUPPORTED_PROTOCOL = 1;
  REJECT_REASON_INSUFFICIENT_BUDGET = 2;
  REJECT_REASON_SERVICE_UNAVAILABLE = 3;
  REJECT_REASON_RATE_LIMITED = 4;
  REJECT_REASON_UNAUTHORIZED = 5;
}
```

### 5.2 RPC 协议

```protobuf
message RpcRequest {
  // 调用 ID
  string call_id = 1;
  
  // 方法名
  string method = 2;
  
  // 参数
  google.protobuf.Any params = 3;
  
  // 元数据
  map<string, string> metadata = 4;
  
  // 超时
  uint32 timeout_ms = 5;
  
  // 预算
  uint64 budget = 6;
}

message RpcResponse {
  // 调用 ID
  string call_id = 1;
  
  // 状态
  RpcStatus status = 2;
  
  // 结果
  google.protobuf.Any result = 3;
  
  // 实际成本
  uint64 actual_cost = 4;
  
  // 处理时间
  uint32 processing_time_ms = 5;
  
  // 错误信息
  RpcError error = 6;
  
  // 元数据
  map<string, string> metadata = 7;
}

enum RpcStatus {
  RPC_STATUS_UNSPECIFIED = 0;
  RPC_STATUS_SUCCESS = 1;
  RPC_STATUS_ERROR = 2;
  RPC_STATUS_TIMEOUT = 3;
  RPC_STATUS_CANCELLED = 4;
  RPC_STATUS_INSUFFICIENT_BUDGET = 5;
}

message RpcError {
  // 错误码
  string code = 1;
  
  // 错误消息
  string message = 2;
  
  // 错误详情
  google.protobuf.Struct details = 3;
  
  // 重试策略
  RetryPolicy retry_policy = 4;
}

message RetryPolicy {
  bool retryable = 1;
  uint32 max_retries = 2;
  uint32 initial_delay_ms = 3;
  uint32 max_delay_ms = 4;
  float delay_multiplier = 5;
}

message RpcStreamFrame {
  // 流 ID
  uint32 stream_id = 1;
  
  // 帧类型
  StreamFrameType type = 2;
  
  // 序列号
  uint64 sequence = 3;
  
  // 标志
  uint32 flags = 4;
  
  // 数据
  bytes data = 5;
}

enum StreamFrameType {
  STREAM_FRAME_TYPE_UNSPECIFIED = 0;
  STREAM_FRAME_TYPE_DATA = 1;
  STREAM_FRAME_TYPE_HEADERS = 2;
  STREAM_FRAME_TYPE_END_STREAM = 3;
  STREAM_FRAME_TYPE_WINDOW_UPDATE = 4;
  STREAM_FRAME_TYPE_PING = 5;
  STREAM_FRAME_TYPE_CANCEL = 6;
  STREAM_FRAME_TYPE_ERROR = 7;
}

message RpcCancel {
  // 调用 ID
  string call_id = 1;
  
  // 取消原因
  string reason = 2;
}
```

---

## 6. 经济协议

### 6.1 通道协议

```protobuf
// economy.proto

message ChannelOpenRequest {
  // 通道 ID
  string channel_id = 1;
  
  // 对端 DID
  string peer_did = 2;
  
  // 本方保证金
  uint64 deposit = 3;
  
  // 超时时间
  uint64 timeout_seconds = 4;
  
  // 结算地址
  string settlement_address = 5;
  
  // 签名
  bytes signature = 6;
}

message ChannelOpenResponse {
  // 是否接受
  bool accepted = 1;
  
  // 对端保证金
  uint64 peer_deposit = 2;
  
  // 通道合约
  ChannelContract contract = 3;
  
  // 错误信息
  string error_message = 4;
}

message ChannelContract {
  // 通道 ID
  string channel_id = 1;
  
  // 参与方 A
  Party party_a = 2;
  
  // 参与方 B
  Party party_b = 3;
  
  // A 的保证金
  uint64 deposit_a = 4;
  
  // B 的保证金
  uint64 deposit_b = 5;
  
  // 超时时间
  uint64 timeout_seconds = 6;
  
  // 创建时间
  google.protobuf.Timestamp created_at = 7;
  
  // A 的签名
  bytes signature_a = 8;
  
  // B 的签名
  bytes signature_b = 9;
}

message Party {
  string did = 1;
  bytes public_key = 2;
  string settlement_address = 3;
}

message ChannelCloseRequest {
  // 通道 ID
  string channel_id = 1;
  
  // 最终余额 A
  uint64 final_balance_a = 2;
  
  // 最终余额 B
  uint64 final_balance_b = 3;
  
  // 最后收据序号
  uint64 last_receipt_sequence = 4;
  
  // 签名
  bytes signature = 5;
}

message ChannelCloseResponse {
  // 是否接受
  bool accepted = 1;
  
  // 结算结果
  SettlementResult settlement = 2;
  
  // 错误信息
  string error_message = 3;
}

message SettlementResult {
  // 通道 ID
  string channel_id = 1;
  
  // A 获得金额
  uint64 amount_a = 2;
  
  // B 获得金额
  uint64 amount_b = 3;
  
  // 总交易数
  uint64 total_transactions = 4;
  
  // 总交易量
  uint64 total_volume = 5;
}
```

### 6.2 收据协议

```protobuf
message ReceiptCreate {
  // 收据 ID
  string receipt_id = 1;
  
  // 通道 ID
  string channel_id = 2;
  
  // 序号
  uint64 sequence = 3;
  
  // 调用方 DID
  string caller_did = 4;
  
  // 服务方 DID
  string provider_did = 5;
  
  // 端点 ID
  string endpoint_id = 6;
  
  // 调用 ID
  string call_id = 7;
  
  // 费用
  uint64 cost = 8;
  
  // 调用前余额
  BalanceSnapshot balance_before = 9;
  
  // 调用后余额
  BalanceSnapshot balance_after = 10;
  
  // 状态
  ReceiptStatus status = 11;
  
  // 时间戳
  google.protobuf.Timestamp timestamp = 12;
}

message BalanceSnapshot {
  uint64 balance_a = 1;
  uint64 balance_b = 2;
}

enum ReceiptStatus {
  RECEIPT_STATUS_UNSPECIFIED = 0;
  RECEIPT_STATUS_SUCCESS = 1;
  RECEIPT_STATUS_PARTIAL_SUCCESS = 2;
  RECEIPT_STATUS_FAILED_NO_CHARGE = 3;
  RECEIPT_STATUS_FAILED_WITH_CHARGE = 4;
  RECEIPT_STATUS_TIMEOUT = 5;
  RECEIPT_STATUS_CANCELLED = 6;
}

message ReceiptSignRequest {
  // 收据
  ReceiptCreate receipt = 1;
  
  // 签名者 DID
  string signer_did = 2;
  
  // 签名
  bytes signature = 3;
}

message ReceiptSignResponse {
  // 是否成功
  bool success = 1;
  
  // 完整收据（含双方签名）
  MicroReceipt receipt = 2;
  
  // 错误信息
  string error_message = 3;
}

message MicroReceipt {
  // 收据 ID
  string receipt_id = 1;
  
  // 通道 ID
  string channel_id = 2;
  
  // 序号
  uint64 sequence = 3;
  
  // 调用方 DID
  string caller_did = 4;
  
  // 服务方 DID
  string provider_did = 5;
  
  // 端点 ID
  string endpoint_id = 6;
  
  // 调用 ID
  string call_id = 7;
  
  // 费用
  uint64 cost = 8;
  
  // 调用前余额
  BalanceSnapshot balance_before = 9;
  
  // 调用后余额
  BalanceSnapshot balance_after = 10;
  
  // 状态
  ReceiptStatus status = 11;
  
  // 时间戳
  google.protobuf.Timestamp timestamp = 12;
  
  // 调用方签名
  bytes signature_caller = 13;
  
  // 服务方签名
  bytes signature_provider = 14;
}
```

---

## 7. 错误处理

### 7.1 错误码定义

```protobuf
// error.proto

message NexaError {
  // 错误码
  ErrorCode code = 1;
  
  // 错误消息
  string message = 2;
  
  // 错误详情
  google.protobuf.Struct details = 3;
  
  // 追踪 ID
  string trace_id = 4;
  
  // 时间戳
  google.protobuf.Timestamp timestamp = 5;
}

enum ErrorCode {
  // 通用错误
  ERROR_CODE_UNSPECIFIED = 0;
  ERROR_CODE_INTERNAL = 1;
  ERROR_CODE_INVALID_REQUEST = 2;
  ERROR_CODE_TIMEOUT = 3;
  ERROR_CODE_CANCELLED = 4;
  
  // 身份层错误 (100-199)
  ERROR_CODE_INVALID_DID = 100;
  ERROR_CODE_DID_NOT_FOUND = 101;
  ERROR_CODE_INVALID_SIGNATURE = 102;
  ERROR_CODE_AUTHENTICATION_FAILED = 103;
  ERROR_CODE_VC_INVALID = 104;
  ERROR_CODE_VC_EXPIRED = 105;
  ERROR_CODE_PERMISSION_DENIED = 106;
  
  // 发现层错误 (200-299)
  ERROR_CODE_REGISTRATION_FAILED = 200;
  ERROR_CODE_NO_MATCHING_SERVICE = 201;
  ERROR_CODE_SERVICE_UNAVAILABLE = 202;
  ERROR_CODE_RATE_LIMITED = 203;
  
  // 传输层错误 (300-399)
  ERROR_CODE_PROTOCOL_NEGOTIATION_FAILED = 300;
  ERROR_CODE_SCHEMA_MISMATCH = 301;
  ERROR_CODE_SERIALIZATION_ERROR = 302;
  ERROR_CODE_CONNECTION_FAILED = 303;
  ERROR_CODE_STREAM_ERROR = 304;
  
  // 经济层错误 (400-499)
  ERROR_CODE_INSUFFICIENT_BALANCE = 400;
  ERROR_CODE_CHANNEL_NOT_FOUND = 401;
  ERROR_CODE_CHANNEL_CLOSED = 402;
  ERROR_CODE_RECEIPT_INVALID = 403;
  ERROR_CODE_BUDGET_EXCEEDED = 404;
  ERROR_CODE_SETTLEMENT_FAILED = 405;
}
```

### 7.2 错误响应格式

```json
{
  "error": {
    "code": "NO_MATCHING_SERVICE",
    "message": "No service found matching the intent",
    "details": {
      "intent": "translate to Martian",
      "similarity_threshold": 0.7,
      "best_match_similarity": 0.45
    },
    "trace_id": "trace-abc123",
    "timestamp": "2026-03-30T07:00:00.000Z"
  }
}
```

---

## 8. 版本协商

### 8.1 版本格式

Nexa-net 使用语义化版本控制：

```
MAJOR.MINOR.PATCH

示例：
1.0.0
1.1.0
2.0.0
```

### 8.2 版本兼容性

| 版本变化 | 兼容性 | 描述 |
|----------|--------|------|
| **MAJOR** | 不兼容 | 破坏性变更 |
| **MINOR** | 向后兼容 | 新增功能 |
| **PATCH** | 向后兼容 | Bug 修复 |

### 8.3 版本协商流程

```
┌─────────────┐                    ┌─────────────┐
│   Client    │                    │   Server    │
│ (v1.2.0)    │                    │ (v1.3.0)    │
└──────┬──────┘                    └──────┬──────┘
       │                                  │
       │ SYN-NEXA                         │
       │ protocol_version: "1.2.0"        │
       │─────────────────────────────────▶│
       │                                  │
       │ ACK-SCHEMA                       │
       │ selected_version: "1.2.0"        │
       │◀─────────────────────────────────│
       │                                  │
       │ 使用 v1.2.0 协议通信              │
       │                                  │
```

---

## 9. 相关文档

### 架构设计

- [整体架构设计](./ARCHITECTURE.md) - 四层架构总览
- [身份与零信任网络层](./IDENTITY_LAYER.md) - 身份协议详细设计
- [语义发现与能力路由层](./DISCOVERY_LAYER.md) - 发现协议详细设计
- [传输与协商协议层](./TRANSPORT_LAYER.md) - 传输协议详细设计
- [资源管理与微交易层](./ECONOMY_LAYER.md) - 经济协议详细设计

### 接口规范

- [API 参考](./API_REFERENCE.md) - API 接口定义
- [开发者接入指南](./DEVELOPER_GUIDE.md) - SDK 使用指南

### 参考资料

- [Protocol Buffers](https://protobuf.dev/)
- [gRPC Protocol](https://grpc.io/docs/)
- [W3C DID Specification](https://www.w3.org/TR/did-core/)
- [W3C Verifiable Credentials](https://www.w3.org/TR/vc-data-model/)