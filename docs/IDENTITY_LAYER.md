# Nexa-net 身份与零信任网络层

> **版本:** v1.0.0-draft | **最后更新:** 2026-03-30
> **所属架构层:** Layer 1 - Identity & Zero-Trust Layer

## 目录

- [1. 概述](#1-概述)
- [2. 去中心化身份标识 (Nexa-DID)](#2-去中心化身份标识-nexa-did)
- [3. 密钥管理](#3-密钥管理)
- [4. 双向 TLS 认证 (mTLS)](#4-双向-tls-认证-mtls)
- [5. 可验证凭证 (Verifiable Credentials)](#5-可验证凭证-verifiable-credentials)
- [6. 信任锚与治理](#6-信任锚与治理)
- [7. 实现规范](#7-实现规范)
- [8. 安全考量](#8-安全考量)
- [9. 相关文档](#9-相关文档)

---

## 1. 概述

### 1.1 设计背景

在 100% 由机器组成的 Nexa-net 网络中，传统的身份认证机制失效：

| 传统机制 | 失效原因 |
|----------|----------|
| **密码认证** | 机器无法记忆和管理密码 |
| **IP 白名单** | Agent 可能动态迁移，IP 不固定 |
| **OAuth 授权** | 需要人工干预，无法自动化 |
| **API Key** | 需要人工分发和管理，存在泄露风险 |

### 1.2 设计目标

Nexa-net 的身份层设计目标：

1. **自主身份 (Self-sovereign Identity)** - Agent 完全控制自己的身份，无需中心化注册
2. **密码学保证** - 所有认证基于非对称密钥签名，无法伪造
3. **零信任架构** - 每次通信都需要验证，不信任任何预设关系
4. **隐私保护** - 最小化信息披露，支持选择性披露凭证

### 1.3 层级职责

```
┌─────────────────────────────────────────────────────────────┐
│                   Layer 1: Identity Layer                   │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                    DID System                        │   │
│  │  - 身份生成与注册                                    │   │
│  │  - DID Document 管理                                 │   │
│  │  - 身份解析                                          │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                   Key Management                     │   │
│  │  - 密钥生成                                          │   │
│  │  - 密钥存储                                          │   │
│  │  - 密钥轮换                                          │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                     mTLS Layer                       │   │
│  │  - 证书生成                                          │   │
│  │  - 双向认证                                          │   │
│  │  - 会话管理                                          │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Verifiable Credentials                  │   │
│  │  - 凭证发放                                          │   │
│  │  - 凭证验证                                          │   │
│  │  - 权限传递                                          │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

---

## 2. 去中心化身份标识 (Nexa-DID)

### 2.1 DID 规范

Nexa-net 采用 W3C DID 规范的扩展实现，称为 **Nexa-DID**。

#### 2.1.1 DID 格式

```
did:nexa:<method-specific-identifier>

示例：
did:nexa:1a2b3c4d5e6f7g8h9i0j1k2l3m4n5o6p7q8r9s0t
```

**组成部分：**
- `did` - 固定前缀，表示去中心化标识符
- `nexa` - 方法名称，标识 Nexa-net 网络
- `<method-specific-identifier>` - 基于公钥哈希的唯一标识符

#### 2.1.2 标识符生成算法

```
输入：公钥 (publicKey)
输出：Nexa-DID 标识符

算法：
1. 选择哈希算法：SHA-256
2. 计算公钥哈希：hash = SHA256(publicKey)
3. 取前 40 字节：identifier = hash[0:40]
4. 编码为十六进制：hex_identifier = hex(identifier)
5. 组合 DID：did = "did:nexa:" + hex_identifier

返回：did
```

**代码示例：**

```python
import hashlib
from cryptography.hazmat.primitives.asymmetric import ed25519

def generate_nexa_did() -> tuple[str, ed25519.Ed25519PrivateKey]:
    """生成新的 Nexa-DID 和对应的私钥"""
    # 1. 生成 Ed25519 密钥对
    private_key = ed25519.Ed25519PrivateKey.generate()
    public_key = private_key.public_key()
    
    # 2. 获取公钥字节
    public_key_bytes = public_key.public_bytes_raw()
    
    # 3. 计算 SHA-256 哈希
    hash_bytes = hashlib.sha256(public_key_bytes).digest()
    
    # 4. 取前 40 字节并编码
    identifier = hash_bytes[:40].hex()
    
    # 5. 组合 DID
    did = f"did:nexa:{identifier}"
    
    return did, private_key
```

### 2.2 DID Document

每个 Nexa-DID 对应一个 DID Document，描述该身份的公钥、服务和验证方法。

#### 2.2.1 Document 结构

```json
{
  "@context": [
    "https://www.w3.org/ns/did/v1",
    "https://nexa-net.org/ns/did/v1"
  ],
  "id": "did:nexa:1a2b3c4d5e6f7g8h9i0j1k2l3m4n5o6p7q8r9s0t",
  "controller": "did:nexa:1a2b3c4d5e6f7g8h9i0j1k2l3m4n5o6p7q8r9s0t",
  "verificationMethod": [
    {
      "id": "did:nexa:1a2b3c4d5e6f7g8h9i0j1k2l3m4n5o6p7q8r9s0t#key-1",
      "type": "Ed25519VerificationKey2020",
      "controller": "did:nexa:1a2b3c4d5e6f7g8h9i0j1k2l3m4n5o6p7q8r9s0t",
      "publicKeyMultibase": "z6MkhaXgBZDvotDkL5257fc8M9Zr3g..."
    }
  ],
  "authentication": [
    "did:nexa:1a2b3c4d5e6f7g8h9i0j1k2l3m4n5o6p7q8r9s0t#key-1"
  ],
  "keyAgreement": [
    {
      "id": "did:nexa:1a2b3c4d5e6f7g8h9i0j1k2l3m4n5o6p7q8r9s0t#key-agreement-1",
      "type": "X25519KeyAgreementKey2020",
      "controller": "did:nexa:1a2b3c4d5e6f7g8h9i0j1k2l3m4n5o6p7q8r9s0t",
      "publicKeyMultibase": "z6LSbysY2Pm8SR..."
    }
  ],
  "service": [
    {
      "id": "did:nexa:1a2b3c4d5e6f7g8h9i0j1k2l3m4n5o6p7q8r9s0t#nexa-proxy",
      "type": "NexaProxyEndpoint",
      "serviceEndpoint": "https://proxy.nexa.net:7070"
    }
  ],
  "created": "2026-03-30T00:00:00Z",
  "updated": "2026-03-30T00:00:00Z"
}
```

#### 2.2.2 字段说明

| 字段 | 类型 | 描述 |
|------|------|------|
| `id` | string | DID 标识符 |
| `controller` | string | 控制者 DID（通常为自身） |
| `verificationMethod` | array | 验证方法（公钥）列表 |
| `authentication` | array | 用于身份认证的验证方法引用 |
| `keyAgreement` | array | 用于密钥协商的验证方法 |
| `service` | array | 服务端点列表 |
| `created` | datetime | 创建时间 |
| `updated` | datetime | 最后更新时间 |

### 2.3 DID 解析

Nexa-net 维护一个分布式的 DID 解析系统，用于从 DID 获取对应的 DID Document。

#### 2.3.1 解析流程

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Client    │────▶│  Resolver   │────▶│   DHT       │
│             │     │             │     │  Storage    │
└─────────────┘     └─────────────┘     └─────────────┘
       │                   │                   │
       │ 1. resolve(did)   │                   │
       │──────────────────▶│                   │
       │                   │ 2. query(hash)    │
       │                   │──────────────────▶│
       │                   │                   │
       │                   │ 3. DID Document   │
       │                   │◀──────────────────│
       │ 4. DID Document   │                   │
       │◀──────────────────│                   │
       │                   │                   │
```

#### 2.3.2 解析器接口

```typescript
interface DIDResolver {
  // 解析 DID Document
  resolve(did: string): Promise<DIDDocument>;
  
  // 解析并验证 DID Document
  resolveWithVerification(did: string): Promise<VerifiedDIDDocument>;
  
  // 缓存管理
  cacheDocument(did: string, document: DIDDocument, ttl: number): void;
  invalidateCache(did: string): void;
}
```

---

## 3. 密钥管理

### 3.1 密钥类型

Nexa-net 支持以下密钥类型：

| 密钥类型 | 用途 | 算法 | 安全等级 |
|----------|------|------|----------|
| **签名密钥** | 身份认证、凭证签名 | Ed25519 | 高 |
| **加密密钥** | 密钥协商、数据加密 | X25519 | 高 |
| **备用密钥** | 密钥轮换、恢复 | Secp256k1 | 高 |

### 3.2 密钥生成

#### 3.2.1 Ed25519 签名密钥

```python
from cryptography.hazmat.primitives.asymmetric import ed25519
from cryptography.hazmat.primitives import serialization

def generate_signing_key() -> tuple[bytes, bytes]:
    """生成 Ed25519 签名密钥对"""
    private_key = ed25519.Ed25519PrivateKey.generate()
    public_key = private_key.public_key()
    
    # 导出为原始字节格式
    private_bytes = private_key.private_bytes_raw()
    public_bytes = public_key.public_bytes_raw()
    
    return private_bytes, public_bytes
```

#### 3.2.2 X25519 密钥协商密钥

```python
from cryptography.hazmat.primitives.asymmetric import x25519

def generate_key_agreement_key() -> tuple[bytes, bytes]:
    """生成 X25519 密钥协商密钥对"""
    private_key = x25519.X25519PrivateKey.generate()
    public_key = private_key.public_key()
    
    private_bytes = private_key.private_bytes_raw()
    public_bytes = public_key.public_bytes_raw()
    
    return private_bytes, public_bytes
```

### 3.3 密钥存储

#### 3.3.1 存储架构

```
┌─────────────────────────────────────────────────────────────┐
│                    Key Storage System                       │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                   Primary Storage                    │   │
│  │  ┌──────────────────────────────────────────────┐   │   │
│  │  │  Encrypted Key File                          │   │   │
│  │  │  - AES-256-GCM 加密                          │   │   │
│  │  │  - PBKDF2 密钥派生                           │   │   │
│  │  │  - 本地文件系统                              │   │   │
│  │  └──────────────────────────────────────────────┘   │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                   Backup Storage                     │   │
│  │  ┌──────────────────────────────────────────────┐   │   │
│  │  │  Recovery Seed (可选)                         │   │   │
│  │  │  - BIP39 助记词                               │   │   │
│  │  │  - 离线备份                                   │   │   │
│  │  └──────────────────────────────────────────────┘   │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                   Memory Cache                       │   │
│  │  - 解密后的密钥缓存（会话期间）                      │   │
│  │  - 自动清理机制                                      │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

#### 3.3.2 密钥文件格式

```yaml
# keyfile.yaml (加密后)
version: 1
encryption:
  algorithm: AES-256-GCM
  kdf: PBKDF2
  kdf_params:
    iterations: 100000
    salt: "<base64-encoded-salt>"
keys:
  signing:
    id: "key-1"
    type: Ed25519
    encrypted_data: "<base64-encoded-encrypted-key>"
  key_agreement:
    id: "key-agreement-1"
    type: X25519
    encrypted_data: "<base64-encoded-encrypted-key>"
metadata:
  created: "2026-03-30T00:00:00Z"
  did: "did:nexa:1a2b3c4d5e6f..."
```

### 3.4 密钥轮换

#### 3.4.1 轮换策略

```
┌─────────────────────────────────────────────────────────────┐
│                    Key Rotation Policy                      │
│                                                             │
│  触发条件：                                                  │
│  - 定期轮换（建议：每 90 天）                                │
│  - 安全事件（密钥泄露嫌疑）                                  │
│  - 算法升级（如 Ed25519 → Ed448）                           │
│                                                             │
│  轮换流程：                                                  │
│  1. 生成新密钥对                                            │
│  2. 更新 DID Document（添加新验证方法）                      │
│  3. 发布更新到 DHT                                          │
│  4. 保留旧密钥 7 天（过渡期）                                │
│  5. 移除旧密钥                                              │
│  6. 安全删除旧密钥文件                                      │
└─────────────────────────────────────────────────────────────┘
```

#### 3.4.2 轮换接口

```typescript
interface KeyRotation {
  // 发起密钥轮换
  initiateRotation(reason: RotationReason): Promise<RotationStatus>;
  
  // 完成密钥轮换
  completeRotation(rotationId: string): Promise<void>;
  
  // 取消密钥轮换
  cancelRotation(rotationId: string): Promise<void>;
  
  // 获取轮换历史
  getRotationHistory(): Promise<RotationRecord[]>;
}
```

---

## 4. 双向 TLS 认证 (mTLS)

### 4.1 mTLS 概述

Nexa-net 强制要求所有节点间通信使用双向 TLS（mTLS）认证。

```
┌─────────────┐                    ┌─────────────┐
│  Nexa-Proxy │                    │  Nexa-Proxy │
│      A      │                    │      B      │
└──────┬──────┘                    └──────┬──────┘
       │                                  │
       │ 1. Client Hello + Client Cert    │
       │─────────────────────────────────▶│
       │                                  │
       │ 2. Server Hello + Server Cert    │
       │◀─────────────────────────────────│
       │                                  │
       │ 3. Certificate Verify            │
       │─────────────────────────────────▶│
       │                                  │
       │ 4. Certificate Verify            │
       │◀─────────────────────────────────│
       │                                  │
       │ 5. Key Exchange                  │
       │◀────────────────────────────────▶│
       │                                  │
       │ 6. Encrypted Data                │
       │◀────────────────────────────────▶│
       │                                  │
```

### 4.2 证书生成

#### 4.2.1 自签名证书

Nexa-net 不依赖传统 CA，每个节点生成自签名证书，证书有效性通过 DID 验证。

```python
from cryptography import x509
from cryptography.hazmat.primitives import hashes
from cryptography.hazmat.primitives.asymmetric import ed25519
import datetime

def generate_nexa_certificate(
    did: str,
    private_key: ed25519.Ed25519PrivateKey,
    public_key: ed25519.Ed25519PublicKey
) -> x509.Certificate:
    """生成 Nexa-net 自签名证书"""
    
    # 证书主题：使用 DID 作为标识
    subject = x509.Name([
        x509.NameAttribute(x509.oid.NameOID.COMMON_NAME, did),
    ])
    
    # 构建证书
    certificate = x509.CertificateBuilder()
        .subject_name(subject)
        .issuer_name(subject)  # 自签名
        .public_key(public_key)
        .serial_number(x509.random_serial_number())
        .not_valid_before(datetime.datetime.utcnow())
        .not_valid_after(datetime.datetime.utcnow() + datetime.timedelta(days=365))
        .add_extension(
            x509.SubjectAlternativeName([
                x509.DNSName(f"{did}.nexa.net"),
            ]),
            critical=False,
        )
        .sign(private_key, hashes.SHA256())
    
    return certificate
```

#### 4.2.2 证书验证逻辑

```python
def verify_nexa_certificate(
    certificate: x509.Certificate,
    did_document: DIDDocument
) -> bool:
    """验证 Nexa-net 证书"""
    
    # 1. 提取证书中的 DID
    cert_did = certificate.subject.get_attributes_for_oid(
        x509.oid.NameOID.COMMON_NAME
    )[0].value
    
    # 2. 验证 DID 格式
    if not cert_did.startswith("did:nexa:"):
        return False
    
    # 3. 从 DID Document 获取公钥
    expected_public_key = extract_public_key_from_did_document(did_document)
    
    # 4. 验证证书公钥与 DID Document 公钥一致
    cert_public_key = certificate.public_key()
    if cert_public_key.public_bytes_raw() != expected_public_key:
        return False
    
    # 5. 验证证书签名（自签名）
    try:
        cert_public_key.verify(
            certificate.signature,
            certificate.tbs_certificate_bytes,
            ed25519.Ed25519SignatureAlgorithm()
        )
        return True
    except Exception:
        return False
```

### 4.3 TLS 配置

#### 4.3.1 Nexa-Proxy TLS 配置

```yaml
# tls_config.yaml
tls:
  version: "TLSv1.3"
  cipher_suites:
    - TLS_AES_256_GCM_SHA384
    - TLS_CHACHA20_POLY1305_SHA256
  certificate_file: "/var/lib/nexa/certs/node.crt"
  private_key_file: "/var/lib/nexa/keys/node.key"
  
  client_authentication:
    required: true
    verify_did: true
    trust_store: "/var/lib/nexa/trust/did_trust_store"
  
  session:
    timeout: 3600  # 1 hour
    max_sessions: 1000
    reuse_enabled: true
```

### 4.4 会话管理

```typescript
interface TLSSessionManager {
  // 创建新会话
  createSession(peerDID: string): Promise<TLSSession>;
  
  // 获取现有会话
  getSession(sessionId: string): Promise<TLSSession | null>;
  
  // 验证会话有效性
  validateSession(session: TLSSession): Promise<boolean>;
  
  // 关闭会话
  closeSession(sessionId: string): Promise<void>;
  
  // 清理过期会话
  cleanupExpiredSessions(): Promise<void>;
}

interface TLSSession {
  sessionId: string;
  peerDID: string;
  establishedAt: Date;
  expiresAt: Date;
  cipherSuite: string;
  keyMaterial: KeyMaterial;
}
```

---

## 5. 可验证凭证 (Verifiable Credentials)

### 5.1 VC 概述

可验证凭证（Verifiable Credentials, VC）用于在 Nexa-net 中传递权限和能力声明。

```
┌─────────────────────────────────────────────────────────────┐
│                 Verifiable Credential Flow                  │
│                                                             │
│  ┌─────────────┐     ┌─────────────┐     ┌─────────────┐   │
│  │   Issuer    │     │   Holder    │     │  Verifier   │   │
│  │ (Trust Anchor)│   │   (Agent)   │     │ (Nexa-Proxy)│   │
│  └──────┬──────┘     └──────┬──────┘     └──────┬──────┘   │
│         │                   │                   │          │
│         │ 1. Issue VC       │                   │          │
│         │──────────────────▶│                   │          │
│         │                   │                   │          │
│         │                   │ 2. Present VC     │          │
│         │                   │──────────────────▶│          │
│         │                   │                   │          │
│         │                   │ 3. Verify VC      │          │
│         │                   │◀──────────────────│          │
│         │                   │                   │          │
│         │ 4. Verify Issuer  │                   │          │
│         │◀──────────────────────────────────────│          │
│         │                   │                   │          │
└─────────────────────────────────────────────────────────────┘
```

### 5.2 VC 结构

#### 5.2.1 标准 VC 格式

```json
{
  "@context": [
    "https://www.w3.org/2018/credentials/v1",
    "https://nexa-net.org/ns/credentials/v1"
  ],
  "id": "vc:nexa:permission:abc123",
  "type": [
    "VerifiableCredential",
    "NexaPermissionCredential"
  ],
  "issuer": {
    "id": "did:nexa:trustanchor001...",
    "name": "Nexa Trust Anchor"
  },
  "issuanceDate": "2026-03-30T00:00:00Z",
  "expirationDate": "2027-03-30T00:00:00Z",
  "credentialSubject": {
    "id": "did:nexa:1a2b3c4d5e6f...",
    "permission": {
      "type": "service_invocation",
      "target": "did:nexa:serviceprovider...",
      "scope": ["audio_to_text", "text_translation"],
      "rateLimit": {
        "maxCalls": 1000,
        "period": "daily"
      }
    }
  },
  "proof": {
    "type": "Ed25519Signature2020",
    "created": "2026-03-30T00:00:00Z",
    "verificationMethod": "did:nexa:trustanchor001...#key-1",
    "proofPurpose": "assertionMethod",
    "proofValue": "z3FXQjecWufY46......"
  }
}
```

### 5.3 VC 类型

Nexa-net 定义以下标准 VC 类型：

| VC 类型 | 用途 | 必要字段 |
|---------|------|----------|
| `NexaPermissionCredential` | 服务调用权限 | `permission.type`, `permission.target`, `permission.scope` |
| `NexaCapabilityCredential` | 能力声明 | `capability.endpoint`, `capability.schema` |
| `NexaReputationCredential` | 信誉评分 | `reputation.score`, `reputation.history` |
| `NexaPaymentCredential` | 支付能力 | `payment.channelId`, `payment.balance` |

### 5.4 VC 发放

#### 5.4.1 发放流程

```python
from datetime import datetime, timedelta
import json

def issue_permission_vc(
    issuer_did: str,
    issuer_private_key: ed25519.Ed25519PrivateKey,
    subject_did: str,
    target_did: str,
    scope: list[str],
    rate_limit: dict
) -> dict:
    """发放权限凭证"""
    
    vc = {
        "@context": [
            "https://www.w3.org/2018/credentials/v1",
            "https://nexa-net.org/ns/credentials/v1"
        ],
        "id": f"vc:nexa:permission:{uuid.uuid4()}",
        "type": ["VerifiableCredential", "NexaPermissionCredential"],
        "issuer": {"id": issuer_did},
        "issuanceDate": datetime.utcnow().isoformat() + "Z",
        "expirationDate": (datetime.utcnow() + timedelta(days=365)).isoformat() + "Z",
        "credentialSubject": {
            "id": subject_did,
            "permission": {
                "type": "service_invocation",
                "target": target_did,
                "scope": scope,
                "rateLimit": rate_limit
            }
        }
    }
    
    # 签名
    vc["proof"] = create_vc_proof(vc, issuer_private_key, issuer_did)
    
    return vc

def create_vc_proof(
    vc: dict,
    private_key: ed25519.Ed25519PrivateKey,
    issuer_did: str
) -> dict:
    """创建 VC 签名证明"""
    
    # 规范化 VC（不含 proof）
    vc_without_proof = {k: v for k, v in vc.items() if k != "proof"}
    canonicalized = json.dumps(vc_without_proof, sort_keys=True)
    
    # 签名
    signature = private_key.sign(canonicalized.encode())
    
    return {
        "type": "Ed25519Signature2020",
        "created": datetime.utcnow().isoformat() + "Z",
        "verificationMethod": f"{issuer_did}#key-1",
        "proofPurpose": "assertionMethod",
        "proofValue": signature.hex()
    }
```

### 5.5 VC 验证

#### 5.5.1 验证流程

```python
def verify_vc(
    vc: dict,
    did_resolver: DIDResolver
) -> tuple[bool, str]:
    """验证可验证凭证"""
    
    # 1. 验证基本结构
    if not validate_vc_structure(vc):
        return False, "Invalid VC structure"
    
    # 2. 验证过期时间
    expiration = datetime.fromisoformat(vc["expirationDate"].replace("Z", "+00:00"))
    if datetime.utcnow() > expiration:
        return False, "VC expired"
    
    # 3. 获取签发者 DID Document
    issuer_did = vc["issuer"]["id"]
    issuer_doc = did_resolver.resolve(issuer_did)
    
    # 4. 验证签名
    if not verify_vc_signature(vc, issuer_doc):
        return False, "Invalid signature"
    
    # 5. 验证签发者信誉（可选）
    if not verify_issuer_reputation(issuer_did):
        return False, "Issuer not trusted"
    
    return True, "VC valid"

def verify_vc_signature(vc: dict, issuer_doc: DIDDocument) -> bool:
    """验证 VC 签名"""
    
    # 获取签发者公钥
    proof = vc["proof"]
    verification_method_id = proof["verificationMethod"]
    
    public_key = extract_key_from_did_document(issuer_doc, verification_method_id)
    
    # 规范化 VC
    vc_without_proof = {k: v for k, v in vc.items() if k != "proof"}
    canonicalized = json.dumps(vc_without_proof, sort_keys=True)
    
    # 验证签名
    try:
        public_key.verify(
            bytes.fromhex(proof["proofValue"]),
            canonicalized.encode()
        )
        return True
    except Exception:
        return False
```

### 5.6 VC Presentation

当 Agent 需要向目标证明权限时，需要创建 VP（Verifiable Presentation）：

```json
{
  "@context": [
    "https://www.w3.org/2018/credentials/v1"
  ],
  "type": ["VerifiablePresentation"],
  "holder": "did:nexa:1a2b3c4d5e6f...",
  "verifiableCredential": [
    { "<embedded VC 1>" },
    { "<embedded VC 2>" }
  ],
  "proof": {
    "type": "Ed25519Signature2020",
    "created": "2026-03-30T00:00:00Z",
    "verificationMethod": "did:nexa:1a2b3c4d5e6f...#key-1",
    "proofPurpose": "authentication",
    "challenge": "random-challenge-string",
    "proofValue": "z3FXQjecWufY46..."
  }
}
```

---

## 6. 信任锚与治理

### 6.1 信任锚 (Trust Anchor)

信任锚是 Nexa-net 中被广泛信任的实体，负责发放初始凭证。

#### 6.1.1 信任锚类型

| 类型 | 职责 | 示例 |
|------|------|------|
| **Root Trust Anchor** | 网络治理、发放其他信任锚凭证 | Nexa-net Foundation |
| **Service Trust Anchor** | 验证服务提供者资质 | 服务审核机构 |
| **Community Trust Anchor** | 社区内权限管理 | Agent 社区管理员 |

#### 6.1.2 信任锚 DID

```
# Root Trust Anchor 示例
did:nexa:trustanchor:root:0000000000000000000000000000000000000000

# Service Trust Anchor 示例
did:nexa:trustanchor:service:abc123def456...
```

### 6.2 信任链

```
┌─────────────────────────────────────────────────────────────┐
│                      Trust Chain                            │
│                                                             │
│  Root Trust Anchor                                          │
│  did:nexa:trustanchor:root:000...                           │
│       │                                                     │
│       │ VC: TrustAnchorCredential                           │
│       ▼                                                     │
│  Service Trust Anchor                                       │
│  did:nexa:trustanchor:service:abc...                        │
│       │                                                     │
│       │ VC: ServiceProviderCredential                       │
│       ▼                                                     │
│  Service Provider                                           │
│  did:nexa:serviceprovider:xyz...                            │
│       │                                                     │
│       │ VC: NexaPermissionCredential                        │
│       ▼                                                     │
│  Agent (Holder)                                             │
│  did:nexa:1a2b3c4d5e6f...                                   │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### 6.3 治理模型

Nexa-net 采用多签名治理模型：

```typescript
interface GovernanceModel {
  // 提案创建
  createProposal(proposer: DID, proposal: Proposal): Promise<ProposalId>;
  
  // 提案投票
  vote(proposalId: ProposalId, voter: DID, vote: VoteType): Promise<void>;
  
  // 提案执行（需要多签名）
  executeProposal(proposalId: ProposalId, signatures: Signature[]): Promise<void>;
  
  // 信任锚管理
  addTrustAnchor(anchor: DID, credentials: VC[]): Promise<void>;
  removeTrustAnchor(anchor: DID, reason: string): Promise<void>;
}
```

---

## 7. 实现规范

### 7.1 接口定义

```typescript
interface IdentityLayerAPI {
  // DID 管理
  did: {
    generate(): Promise<DIDKeyPair>;
    resolve(did: string): Promise<DIDDocument>;
    update(did: string, document: DIDDocument): Promise<void>;
    deactivate(did: string): Promise<void>;
  };
  
  // 密钥管理
  keys: {
    generate(type: KeyType): Promise<KeyPair>;
    store(keyId: string, privateKey: bytes): Promise<void>;
    retrieve(keyId: string): Promise<bytes>;
    rotate(keyId: string): Promise<KeyPair>;
  };
  
  // TLS 管理
  tls: {
    generateCertificate(did: string): Promise<Certificate>;
    createSession(peerDID: string): Promise<TLSSession>;
    validateSession(sessionId: string): Promise<boolean>;
  };
  
  // VC 管理
  vc: {
    issue(issuer: DID, subject: DID, claims: Claims): Promise<VC>;
    verify(vc: VC): Promise<VerificationResult>;
    present(holder: DID, vcs: VC[], challenge: string): Promise<VP>;
  };
}
```

### 7.2 错误码

| 错误码 | 描述 | 处理建议 |
|--------|------|----------|
| `ID001` | DID 格式无效 | 检查 DID 字符串格式 |
| `ID002` | DID Document 解析失败 | 检查网络连接或 DHT 状态 |
| `ID003` | 密钥签名验证失败 | 检查密钥是否匹配 |
| `ID004` | 证书验证失败 | 检查证书是否过期或被篡改 |
| `ID005` | VC 签名无效 | 检查签发者公钥 |
| `ID006` | VC 已过期 | 重新申请凭证 |
| `ID007` | 权限不足 | 检查 VC 权限范围 |
| `ID008` | 信任锚不可信 | 检查信任链 |

---

## 8. 安全考量

### 8.1 威胁模型

| 威胁 | 风险等级 | 缓解措施 |
|------|----------|----------|
| **私钥泄露** | 高 | AES-256 加密存储、内存安全清理 |
| **DID 劫持** | 高 | DID Document 签名验证、密钥轮换 |
| **中间人攻击** | 高 | 强制 mTLS、证书绑定 |
| **VC 伪造** | 中 | 签名验证、信任链验证 |
| **信任锚滥用** | 中 | 多签名治理、审计日志 |
| **密钥轮换攻击** | 低 | 过渡期保留旧密钥、撤销机制 |

### 8.2 安全最佳实践

1. **密钥存储**
   - 使用 AES-256-GCM 加密
   - PBKDF2 密钥派生（至少 100,000 次迭代）
   - 定期检查密钥文件完整性

2. **证书管理**
   - 证书有效期不超过 365 天
   - 使用 TLS 1.3
   - 禁用弱密码套件

3. **VC 管理**
   - 凭证有效期不超过 365 天
   - 使用最小权限原则
   - 定期审计凭证发放记录

---

## 9. 相关文档

### 上层架构

- [整体架构设计](./ARCHITECTURE.md) - 四层架构总览

### 下层依赖

- [语义发现与能力路由层](./DISCOVERY_LAYER.md) - Layer 2 设计
- [传输与协商协议层](./TRANSPORT_LAYER.md) - Layer 3 设计

### 相关规范

- [协议规范](./PROTOCOL_SPEC.md) - 身份相关协议定义
- [安全设计](./SECURITY.md) - 安全威胁模型与审计
- [API 参考](./API_REFERENCE.md) - 身份层 API 定义

### 参考资料

- [W3C DID Specification](https://www.w3.org/TR/did-core/)
- [W3C Verifiable Credentials](https://www.w3.org/TR/vc-data-model/)
- [RFC 8446 - TLS 1.3](https://datatracker.ietf.org/doc/html/rfc8446)