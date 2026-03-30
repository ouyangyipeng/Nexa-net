# Nexa-net 语义发现与能力路由层

> **版本:** v1.0.0-draft | **最后更新:** 2026-03-30
> **所属架构层:** Layer 2 - Semantic Discovery & Capability Routing Layer

## 目录

- [1. 概述](#1-概述)
- [2. 能力清单 (Capability Schema)](#2-能力清单-capability-schema)
- [3. 语义向量化](#3-语义向量化)
- [4. 分布式语义哈希表 (Semantic DHT)](#4-分布式语义哈希表-semantic-dht)
- [5. 语义路由算法](#5-语义路由算法)
- [6. 节点状态管理](#6-节点状态管理)
- [7. 实现规范](#7-实现规范)
- [8. 性能考量](#8-性能考量)
- [9. 相关文档](#9-相关文档)

---

## 1. 概述

### 1.1 设计背景

传统互联网使用 DNS（域名系统）进行服务发现，Agent 需要知道精确的 URL 才能调用服务。这种方式在 M2M 场景下存在以下问题：

| 问题 | 传统 DNS 方案 | 影响 |
|------|---------------|------|
| **寻址方式** | 需要精确 URL | Agent 需要预先知道服务地址 |
| **语义缺失** | 无语义信息 | 无法根据意图自动匹配服务 |
| **动态性差** | 静态映射 | 无法反映服务实时状态 |
| **能力描述** | 无标准化 | Agent 无法理解服务能力 |

### 1.2 设计目标

Nexa-net 的语义发现层设计目标：

1. **意图驱动 (Intent-Driven)** - Agent 通过描述意图而非 URL 寻找服务
2. **语义匹配 (Semantic Matching)** - 基于向量相似度自动匹配最合适的服务
3. **动态发现 (Dynamic Discovery)** - 实时反映服务状态和能力变化
4. **标准化描述 (Standardized Description)** - 统一的能力 Schema 格式

### 1.3 核心概念

```
┌─────────────────────────────────────────────────────────────┐
│              Semantic Discovery Core Concepts               │
│                                                             │
│  Intent (意图)                                              │
│  └─────────────────────────────────────────────────────    │
│  Agent 发出的服务请求描述，如：                              │
│  "translate English PDF to Chinese and extract key metrics" │
│                                                             │
│  Capability (能力)                                          │
│  └─────────────────────────────────────────────────────    │
│  服务提供者注册的能力描述，包含：                            │
│  - Endpoint: 服务名称                                       │
│  - Input/Output Schema: 数据格式                            │
│  - Cost: 调用成本                                           │
│  - Rate Limit: 速率限制                                     │
│                                                             │
│  Semantic Vector (语义向量)                                 │
│  └─────────────────────────────────────────────────────    │
│  通过 Embedding 模型将文本转换为高维向量：                   │
│  Intent → V_intent, Capability → V_capability              │
│                                                             │
│  Routing (路由)                                             │
│  └─────────────────────────────────────────────────────    │
│  计算 V_intent 与 V_capability 的相似度，选择最优服务       │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### 1.4 层级架构

```
┌─────────────────────────────────────────────────────────────┐
│                   Layer 2: Discovery Layer                  │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Capability Registry                     │   │
│  │  - Schema 注册与验证                                 │   │
│  │  - 能力索引维护                                      │   │
│  │  - 版本管理                                          │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Semantic Vectorizer                     │   │
│  │  - Embedding 模型管理                                │   │
│  │  - 向量生成                                          │   │
│  │  - 向量索引                                          │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Semantic DHT                            │   │
│  │  - 分布式存储                                        │   │
│  │  - 向量检索                                          │   │
│  │  - 节点同步                                          │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Routing Engine                          │   │
│  │  - 相似度计算                                        │   │
│  │  - 多因素权重                                        │   │
│  │  - 候选排序                                          │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Node Status Manager                     │   │
│  │  - 负载监控                                          │   │
│  │  - 健康检查                                          │   │
│  │  - 状态同步                                          │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

---

## 2. 能力清单 (Capability Schema)

### 2.1 Schema 规范

Nexa-net 的能力清单基于扩展的 OpenAPI 3.1 和 Model Context Protocol (MCP) 规范。

#### 2.1.1 Schema 结构

```yaml
# capability_schema.yaml
nexa_capability:
  version: "1.0.0"
  metadata:
    did: "did:nexa:serviceprovider123..."
    name: "Advanced Translation Service"
    description: "Professional document translation with format preservation"
    tags: ["translation", "document", "nlp"]
    
  endpoints:
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
            description: "Source document to translate"
          source_language:
            type: string
            enum: ["en", "ja", "ko", "fr", "de", "es"]
            description: "Source language code"
          target_language:
            type: string
            enum: ["zh", "en", "ja", "ko"]
            description: "Target language code"
          preserve_formatting:
            type: boolean
            default: true
            description: "Whether to preserve document formatting"
        required: ["document", "source_language", "target_language"]
        
      output_schema:
        type: object
        properties:
          translated_document:
            type: binary
            format: application/pdf
            description: "Translated document"
          metadata:
            type: object
            properties:
              pages_processed:
                type: integer
              characters_translated:
                type: integer
              processing_time_ms:
                type: integer
              
      cost:
        model: "per_page"
        base_price: 5  # Nexa-Tokens per page
        modifiers:
          - condition: "preserve_formatting == true"
            multiplier: 1.5
          - condition: "target_language == 'zh'"
            multiplier: 1.2
            
      rate_limit:
        max_concurrent: 5
        max_per_minute: 30
        max_per_day: 1000
        
      quality:
        accuracy_score: 0.95
        avg_latency_ms: 2000
        availability: 0.99
        
    - id: "extract_metrics"
      name: "Key Metrics Extraction"
      description: "Extract numerical metrics and KPIs from documents"
      input_schema:
        type: object
        properties:
          document:
            type: binary
            description: "Document to analyze"
          metrics_types:
            type: array
            items:
              type: string
              enum: ["financial", "operational", "performance"]
        required: ["document"]
        
      output_schema:
        type: object
        properties:
          metrics:
            type: array
            items:
              type: object
              properties:
                name: string
                value: number
                unit: string
                confidence: number
                
      cost:
        model: "per_document"
        base_price: 10
        
  semantic_embedding:
    model: "all-MiniLM-L6-v2"
    vector_dimension: 384
    precomputed_vectors:
      - endpoint_id: "translate_document"
        vector: [0.12, -0.34, 0.56, ...]  # 384 dimensions
      - endpoint_id: "extract_metrics"
        vector: [0.23, 0.45, -0.67, ...]
```

### 2.2 Schema 字段详解

#### 2.2.1 元数据字段

| 字段 | 类型 | 必需 | 描述 |
|------|------|------|------|
| `did` | string | ✅ | 服务提供者的 Nexa-DID |
| `name` | string | ✅ | 服务名称（人类可读） |
| `description` | string | ✅ | 服务描述（用于语义匹配） |
| `tags` | array | ❌ | 分类标签 |
| `version` | string | ✅ | Schema 版本 |

#### 2.2.2 Endpoint 字段

| 字段 | 类型 | 必需 | 描述 |
|------|------|------|------|
| `id` | string | ✅ | Endpoint 唯一标识 |
| `name` | string | ✅ | Endpoint 名称 |
| `description` | string | ✅ | 功能描述（用于语义匹配） |
| `input_schema` | JSON Schema | ✅ | 输入参数定义 |
| `output_schema` | JSON Schema | ✅ | 输出结果定义 |
| `cost` | object | ✅ | 成本模型 |
| `rate_limit` | object | ❌ | 速率限制 |
| `quality` | object | ❌ | 质量指标 |

#### 2.2.3 成本模型

```typescript
interface CostModel {
  // 计费模式
  model: "per_call" | "per_page" | "per_token" | "per_byte" | "per_second";
  
  // 基础价格（Nexa-Tokens）
  base_price: number;
  
  // 价格修饰器
  modifiers?: CostModifier[];
}

interface CostModifier {
  // 条件表达式
  condition: string;  // JSONLogic 表达式
  
  // 价格乘数
  multiplier: number;
}
```

### 2.3 Schema 注册流程

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Service   │     │  Nexa-Proxy │     │  Validator  │     │  Supernode  │
│  Provider   │     │             │     │             │     │   Registry  │
└──────┬──────┘     └──────┬──────┘     └──────┬──────┘     └──────┬──────┘
       │                   │                   │                   │
       │ 1. Schema File    │                   │                   │
       │──────────────────▶│                   │                   │
       │                   │                   │                   │
       │                   │ 2. Validate Schema│                   │
       │                   │──────────────────▶│                   │
       │                   │                   │                   │
       │                   │ 3. Validation OK  │                   │
       │                   │◀──────────────────│                   │
       │                   │                   │                   │
       │                   │ 4. Compute Embedding                   │
       │                   │──────────────────────────────────────▶│
       │                   │                   │                   │
       │                   │ 5. Register + Vector                   │
       │                   │──────────────────────────────────────▶│
       │                   │                   │                   │
       │                   │ 6. Registration Confirmed              │
       │                   │◀──────────────────────────────────────│
       │                   │                   │                   │
       │ 7. Ready          │                   │                   │
       │◀──────────────────│                   │                   │
       │                   │                   │                   │
```

### 2.4 Schema 验证规则

```python
def validate_capability_schema(schema: dict) -> tuple[bool, list[str]]:
    """验证能力清单 Schema"""
    
    errors = []
    
    # 1. 验证必需字段
    required_fields = ["version", "metadata", "endpoints"]
    for field in required_fields:
        if field not in schema:
            errors.append(f"Missing required field: {field}")
    
    # 2. 验证 metadata
    if "metadata" in schema:
        metadata = schema["metadata"]
        if "did" not in metadata:
            errors.append("Missing metadata.did")
        elif not validate_did_format(metadata["did"]):
            errors.append("Invalid DID format")
            
        if "description" not in metadata:
            errors.append("Missing metadata.description")
    
    # 3. 验证 endpoints
    if "endpoints" in schema:
        for endpoint in schema["endpoints"]:
            # 验证 endpoint 必需字段
            endpoint_required = ["id", "input_schema", "output_schema", "cost"]
            for field in endpoint_required:
                if field not in endpoint:
                    errors.append(f"Endpoint missing field: {field}")
            
            # 验证 JSON Schema 格式
            if "input_schema" in endpoint:
                if not validate_json_schema(endpoint["input_schema"]):
                    errors.append(f"Invalid input_schema for endpoint {endpoint.get('id')}")
            
            # 验证成本模型
            if "cost" in endpoint:
                if not validate_cost_model(endpoint["cost"]):
                    errors.append(f"Invalid cost model for endpoint {endpoint.get('id')}")
    
    return len(errors) == 0, errors
```

---

## 3. 语义向量化

### 3.1 Embedding 模型选择

Nexa-net 推荐使用轻量级 Embedding 模型，平衡性能与精度：

| 模型 | 维度 | 速度 | 精度 | 适用场景 |
|------|------|------|------|----------|
| **all-MiniLM-L6-v2** | 384 | 快 | 中 | 默认选择，适合大多数场景 |
| **all-mpnet-base-v2** | 768 | 中 | 高 | 高精度需求 |
| **multilingual-e5-small** | 384 | 快 | 中 | 多语言场景 |
| **bge-small-en-v1.5** | 384 | 快 | 高 | 英文专用 |

### 3.2 向量生成流程

```
┌─────────────────────────────────────────────────────────────┐
│                   Vector Generation Flow                    │
│                                                             │
│  输入文本                                                    │
│  "translate English PDF to Chinese"                         │
│       │                                                     │
│       ▼                                                     │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Text Preprocessing                      │   │
│  │  - 清理特殊字符                                      │   │
│  │  - 统一大小写                                        │   │
│  │  - 分词                                              │   │
│  └─────────────────────────────────────────────────────┘   │
│       │                                                     │
│       ▼                                                     │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Embedding Model                         │   │
│  │  - 加载预训练模型                                    │   │
│  │  - 执行推理                                          │   │
│  │  - 输出向量                                          │   │
│  └─────────────────────────────────────────────────────┘   │
│       │                                                     │
│       ▼                                                     │
│  输出向量                                                    │
│  [0.12, -0.34, 0.56, 0.78, ..., 0.23]  # 384 维            │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### 3.3 向量生成实现

```python
from sentence_transformers import SentenceTransformer
import numpy as np

class SemanticVectorizer:
    """语义向量化器"""
    
    def __init__(self, model_name: str = "all-MiniLM-L6-v2"):
        self.model = SentenceTransformer(model_name)
        self.dimension = self.model.get_sentence_embedding_dimension()
    
    def vectorize_intent(self, intent: str) -> np.ndarray:
        """将意图文本转换为向量"""
        # 预处理
        processed_intent = self._preprocess_text(intent)
        
        # 生成向量
        vector = self.model.encode(processed_intent, normalize_embeddings=True)
        
        return vector
    
    def vectorize_capability(self, capability: dict) -> dict[str, np.ndarray]:
        """将能力清单转换为向量字典"""
        vectors = {}
        
        # 向量化整体描述
        overall_desc = f"{capability['metadata']['name']}: {capability['metadata']['description']}"
        vectors["overall"] = self.model.encode(overall_desc, normalize_embeddings=True)
        
        # 向量化每个 endpoint
        for endpoint in capability["endpoints"]:
            endpoint_desc = f"{endpoint['name']}: {endpoint['description']}"
            vectors[endpoint["id"]] = self.model.encode(
                endpoint_desc, 
                normalize_embeddings=True
            )
        
        return vectors
    
    def _preprocess_text(self, text: str) -> str:
        """文本预处理"""
        # 移除多余空格
        text = " ".join(text.split())
        
        # 移除特殊字符（保留基本标点）
        text = text.replace("\n", " ").replace("\t", " ")
        
        return text.strip()
```

### 3.4 向量索引结构

为了高效检索，Nexa-net 使用近似最近邻（ANN）索引：

```typescript
interface VectorIndex {
  // 索引类型
  type: "HNSW" | "IVF" | "Flat";
  
  // 向量维度
  dimension: number;
  
  // 索引参数
  params: {
    // HNSW 参数
    M?: number;          // 每层连接数
    efConstruction?: number;  // 构建时搜索宽度
    
    // IVF 参数
    nlist?: number;      // 聚类中心数量
  };
  
  // 元数据关联
  metadata: Map<VectorId, EndpointMetadata>;
}

interface EndpointMetadata {
  did: string;
  endpointId: string;
  cost: CostModel;
  quality: QualityMetrics;
  lastUpdated: Date;
}
```

---

## 4. 分布式语义哈希表 (Semantic DHT)

### 4.1 DHT 架构

Nexa-net 使用改进的分布式哈希表（DHT）存储语义向量索引：

```
┌─────────────────────────────────────────────────────────────┐
│                    Semantic DHT Architecture                │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                  Supernode Cluster                   │   │
│  │                                                      │   │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐│   │
│  │  │  Supernode 1 │  │  Supernode 2 │  │  Supernode 3 ││   │
│  │  │              │  │              │  │              ││   │
│  │  │ Vector Index │  │ Vector Index │  │ Vector Index ││   │
│  │  │ (Shard 1-3)  │  │ (Shard 4-6)  │  │ (Shard 7-9)  ││   │
│  │  │              │  │              │  │              ││   │
│  │  │ Metadata DB  │  │ Metadata DB  │  │ Metadata DB  ││   │
│  │  └──────────────┘  └──────────────┘  └──────────────┘│   │
│  │                                                      │   │
│  │         ──────── Replication & Sync ────────         │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                    Edge Nodes                        │   │
│  │                                                      │   │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐│   │
│  │  │ Nexa-Proxy A │  │ Nexa-Proxy B │  │ Nexa-Proxy C ││   │
│  │  │              │  │              │  │              ││   │
│  │  │ Local Cache  │  │ Local Cache  │  │ Local Cache  ││   │
│  │  │ (Hot Entries)│  │ (Hot Entries)│  │ (Hot Entries)││   │
│  │  └──────────────┘  └──────────────┘  └──────────────┘│   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

### 4.2 分片策略

#### 4.2.1 向量分片

使用向量聚类进行分片，确保相似向量在同一分片：

```python
def shard_by_cluster(vectors: list[np.ndarray], n_shards: int) -> dict[int, list[int]]:
    """基于向量聚类进行分片"""
    
    # 1. 执行 K-Means 聚类
    from sklearn.cluster import KMeans
    kmeans = KMeans(n_clusters=n_shards, random_state=42)
    cluster_labels = kmeans.fit_predict(vectors)
    
    # 2. 分配向量到分片
    shards = {}
    for i, label in enumerate(cluster_labels):
        if label not in shards:
            shards[label] = []
        shards[label].append(i)
    
    return shards, kmeans.cluster_centers_
```

#### 4.2.2 分片路由

```typescript
interface ShardRouter {
  // 根据向量确定目标分片
  routeToShard(queryVector: number[]): ShardId;
  
  // 获取分片位置
  getShardLocation(shardId: ShardId): SupernodeAddress;
  
  // 分片健康检查
  checkShardHealth(shardId: ShardId): HealthStatus;
}

// 路由算法
function routeToShard(queryVector: number[], clusterCenters: number[][]): number {
  // 计算与各聚类中心的距离
  let minDistance = Infinity;
  let targetShard = 0;
  
  for (let i = 0; i < clusterCenters.length; i++) {
    const distance = euclideanDistance(queryVector, clusterCenters[i]);
    if (distance < minDistance) {
      minDistance = distance;
      targetShard = i;
    }
  }
  
  return targetShard;
}
```

### 4.3 DHT 操作

#### 4.3.1 注册能力

```python
def register_capability(
    did: str,
    capability: dict,
    vectors: dict[str, np.ndarray],
    dht_client: DHTClient
) -> bool:
    """注册能力到 DHT"""
    
    # 1. 为每个 endpoint 创建索引条目
    for endpoint_id, vector in vectors.items():
        if endpoint_id == "overall":
            continue
            
        # 创建元数据
        metadata = {
            "did": did,
            "endpoint_id": endpoint_id,
            "cost": capability["endpoints"][endpoint_id]["cost"],
            "quality": capability["endpoints"][endpoint_id].get("quality", {}),
            "registered_at": datetime.utcnow().isoformat()
        }
        
        # 存储到 DHT
        entry_key = f"capability:{did}:{endpoint_id}"
        dht_client.put(entry_key, {
            "vector": vector.tolist(),
            "metadata": metadata
        })
    
    # 2. 更新向量索引
    dht_client.update_vector_index(vectors, metadata)
    
    return True
```

#### 4.3.2 查询能力

```python
def query_capabilities(
    intent_vector: np.ndarray,
    top_k: int,
    dht_client: DHTClient
) -> list[dict]:
    """查询匹配的能力"""
    
    # 1. 确定目标分片
    target_shards = dht_client.route_to_shards(intent_vector, n_shards=3)
    
    # 2. 在各分片执行 ANN 搜索
    results = []
    for shard_id in target_shards:
        shard_results = dht_client.search_in_shard(
            shard_id, 
            intent_vector, 
            top_k=top_k
        )
        results.extend(shard_results)
    
    # 3. 合并并排序结果
    results.sort(key=lambda x: x["similarity"], reverse=True)
    
    return results[:top_k]
```

### 4.4 数据同步

#### 4.4.1 同步协议

```
Supernode A                    Supernode B
     │                              │
     │ 1. Sync Request              │
     │ (last_sync_timestamp)        │
     │─────────────────────────────▶│
     │                              │
     │ 2. Sync Response             │
     │ (changes since timestamp)    │
     │◀─────────────────────────────│
     │                              │
     │ 3. Apply Changes             │
     │                              │
     │ 4. Ack                       │
     │─────────────────────────────▶│
     │                              │
```

#### 4.4.2 同步内容

```typescript
interface SyncMessage {
  // 同步类型
  type: "full" | "delta";
  
  // 变更记录
  changes: ChangeRecord[];
  
  // 同步时间戳
  timestamp: Date;
}

interface ChangeRecord {
  // 操作类型
  operation: "insert" | "update" | "delete";
  
  // 条目键
  key: string;
  
  // 条目数据（insert/update）
  data?: VectorEntry;
  
  // 时间戳
  timestamp: Date;
}
```

---

## 5. 语义路由算法

### 5.1 路由流程

```
┌─────────────────────────────────────────────────────────────┐
│                    Semantic Routing Flow                    │
│                                                             │
│  ┌─────────────┐                                            │
│  │   Intent    │  "translate English PDF to Chinese"       │
│  └─────────────┘                                            │
│       │                                                     │
│       ▼                                                     │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Step 1: Vectorization                   │   │
│  │  Intent → V_intent (384-dim vector)                  │   │
│  └─────────────────────────────────────────────────────┘   │
│       │                                                     │
│       ▼                                                     │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Step 2: ANN Search                      │   │
│  │  Find top-k candidates by cosine similarity          │   │
│  │  Candidates: [Service_A, Service_B, Service_C]       │   │
│  └─────────────────────────────────────────────────────┘   │
│       │                                                     │
│       ▼                                                     │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Step 3: Threshold Filter                │   │
│  │  Filter by similarity > τ (default: 0.7)             │   │
│  │  Filtered: [Service_A, Service_B]                    │   │
│  └─────────────────────────────────────────────────────┘   │
│       │                                                     │
│       ▼                                                     │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Step 4: Multi-factor Ranking            │   │
│  │  W = α·Similarity - β·Latency - γ·Cost               │   │
│  │  Ranked: [Service_B (W=0.85), Service_A (W=0.72)]    │   │
│  └─────────────────────────────────────────────────────┘   │
│       │                                                     │
│       ▼                                                     │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Step 5: Final Selection                 │   │
│  │  Return best candidate: Service_B                    │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### 5.2 相似度计算

#### 5.2.1 余弦相似度

$$\text{Similarity} = \frac{V_{req} \cdot V_{node}}{||V_{req}|| \times ||V_{node}||}$$

```python
def cosine_similarity(v1: np.ndarray, v2: np.ndarray) -> float:
    """计算余弦相似度"""
    # 向量已归一化，直接计算点积
    return np.dot(v1, v2)
```

#### 5.2.2 阈值过滤

```python
def filter_by_threshold(
    candidates: list[dict],
    threshold: float = 0.7
) -> list[dict]:
    """根据相似度阈值过滤候选"""
    return [c for c in candidates if c["similarity"] >= threshold]
```

### 5.3 多因素权重计算

#### 5.3.1 权重公式

$$W = \alpha \cdot \text{Similarity} - \beta \cdot \text{Latency} - \gamma \cdot \text{Cost} + \delta \cdot \text{Quality}$$

**参数说明：**
- $\alpha$ - 相似度权重（默认 0.5）
- $\beta$ - 延迟惩罚系数（默认 0.001，单位 ms）
- $\gamma$ - 成本惩罚系数（默认 0.01，单位 Token）
- $\delta$ - 质量奖励系数（默认 0.2）

#### 5.3.2 权重计算实现

```python
def calculate_routing_weight(
    candidate: dict,
    params: RoutingParams
) -> float:
    """计算路由权重"""
    
    similarity = candidate["similarity"]
    latency = candidate.get("latency_ms", 100)  # 默认 100ms
    cost = candidate.get("cost", 10)  # 默认 10 Tokens
    quality = candidate.get("quality_score", 0.8)  # 默认 0.8
    
    # 计算权重
    weight = (
        params.alpha * similarity
        - params.beta * latency
        - params.gamma * cost
        + params.delta * quality
    )
    
    return weight

class RoutingParams:
    alpha: float = 0.5      # 相似度权重
    beta: float = 0.001     # 延迟惩罚（每 ms）
    gamma: float = 0.01     # 成本惩罚（每 Token）
    delta: float = 0.2      # 质量奖励
```

### 5.4 动态阈值调整

阈值 $\tau$ 根据网络状态动态调整：

```python
def adjust_threshold(
    base_threshold: float,
    network_load: float,
    candidate_count: int
) -> float:
    """动态调整相似度阈值"""
    
    # 网络负载高时降低阈值，增加候选
    load_adjustment = -0.1 * network_load
    
    # 候选数量少时降低阈值
    count_adjustment = -0.05 * max(0, 5 - candidate_count)
    
    # 计算调整后阈值
    adjusted = base_threshold + load_adjustment + count_adjustment
    
    # 确保阈值在合理范围
    return max(0.5, min(0.9, adjusted))
```

### 5.5 路由决策接口

```typescript
interface RoutingEngine {
  // 执行路由
  route(intent: Intent, params?: RoutingParams): Promise<RoutingResult>;
  
  // 获取候选列表
  getCandidates(intentVector: number[], topK: number): Promise<Candidate[]>;
  
  // 计算权重
  calculateWeight(candidate: Candidate, params: RoutingParams): number;
  
  // 更新路由参数
  updateParams(params: Partial<RoutingParams>): void;
}

interface RoutingResult {
  // 选中的服务
  selected: Candidate;
  
  // 所有候选
  candidates: Candidate[];
  
  // 路由决策详情
  decision: {
    similarity: number;
    weight: number;
    threshold: number;
    latencyEstimate: number;
    costEstimate: number;
  };
}

interface Candidate {
  did: string;
  endpointId: string;
  similarity: number;
  latencyMs: number;
  cost: number;
  qualityScore: number;
  metadata: EndpointMetadata;
}
```

---

## 6. 节点状态管理

### 6.1 状态指标

```typescript
interface NodeStatus {
  // 基本状态
  did: string;
  online: boolean;
  lastHeartbeat: Date;
  
  // 负载指标
  load: {
    cpu: number;          // CPU 使用率 (0-1)
    memory: number;       // 内存使用率 (0-1)
    concurrentCalls: number;  // 当前并发调用数
    queueLength: number;  // 等待队列长度
  };
  
  // 性能指标
  performance: {
    avgLatencyMs: number;     // 平均延迟
    p99LatencyMs: number;     // P99 延迟
    successRate: number;      // 成功率 (0-1)
    throughput: number;       // 每秒处理数
  };
  
  // 经济指标
  economy: {
    availableBalance: number;  // 可用余额
    pendingPayments: number;   // 待结算金额
    channelStatus: "open" | "closing" | "closed";
  };
}
```

### 6.2 心跳机制

```
┌─────────────┐                    ┌─────────────┐
│  Nexa-Proxy │                    │  Supernode  │
└──────┬──────┘                    └──────┬──────┘
       │                                  │
       │ Heartbeat (every 30s)            │
       │─────────────────────────────────▶│
       │                                  │
       │ {                                │
       │   did: "...",                    │
       │   load: {...},                   │
       │   performance: {...}             │
       │ }                                │
       │                                  │
       │ ACK                              │
       │◀─────────────────────────────────│
       │                                  │
       │                                  │
       │ Timeout (90s no heartbeat)       │
       │                                  │
       │                                  │ Mark as offline
       │                                  │
```

### 6.3 健康检查

```python
def check_node_health(status: NodeStatus) -> HealthCheckResult:
    """检查节点健康状态"""
    
    issues = []
    
    # 1. 检查心跳
    heartbeat_age = (datetime.utcnow() - status.lastHeartbeat).total_seconds()
    if heartbeat_age > 90:
        issues.append("Heartbeat timeout")
    
    # 2. 检查负载
    if status.load.cpu > 0.9:
        issues.append("CPU overloaded")
    if status.load.memory > 0.9:
        issues.append("Memory overloaded")
    if status.load.concurrentCalls > status.rate_limit.max_concurrent:
        issues.append("Concurrent calls exceeded")
    
    # 3. 检查性能
    if status.performance.successRate < 0.8:
        issues.append("Low success rate")
    if status.performance.p99LatencyMs > 5000:
        issues.append("High latency")
    
    # 4. 检查经济状态
    if status.economy.availableBalance < 10:
        issues.append("Low balance")
    
    return HealthCheckResult(
        healthy=len(issues) == 0,
        issues=issues,
        score=calculate_health_score(status)
    )

def calculate_health_score(status: NodeStatus) -> float:
    """计算健康评分 (0-1)"""
    
    # 心跳评分
    heartbeat_age = (datetime.utcnow() - status.lastHeartbeat).total_seconds()
    heartbeat_score = max(0, 1 - heartbeat_age / 90)
    
    # 负载评分
    load_score = 1 - max(status.load.cpu, status.load.memory)
    
    # 性能评分
    performance_score = (
        status.performance.successRate * 0.5
        + max(0, 1 - status.performance.p99LatencyMs / 5000) * 0.5
    )
    
    # 综合评分
    return heartbeat_score * 0.3 + load_score * 0.3 + performance_score * 0.4
```

### 6.4 状态同步

```typescript
interface StatusSync {
  // 推送状态更新
  pushStatusUpdate(status: NodeStatus): Promise<void>;
  
  // 拉取节点状态
  pullNodeStatus(did: string): Promise<NodeStatus>;
  
  // 批量获取状态
  batchGetStatus(dids: string[]): Promise<Map<string, NodeStatus>>;
  
  // 订阅状态变更
  subscribeStatusChanges(dids: string[], callback: StatusCallback): Promise<void>;
}
```

---

## 7. 实现规范

### 7.1 接口定义

```typescript
interface DiscoveryLayerAPI {
  // 能力注册
  capability: {
    register(schema: CapabilitySchema): Promise<RegistrationResult>;
    unregister(did: string, endpointId: string): Promise<void>;
    update(did: string, endpointId: string, updates: Partial<EndpointSchema>): Promise<void>;
    get(did: string, endpointId: string): Promise<EndpointSchema>;
  };
  
  // 语义路由
  routing: {
    route(intent: Intent, params?: RoutingParams): Promise<RoutingResult>;
    getCandidates(intent: string, topK?: number): Promise<Candidate[]>;
    setRoutingParams(params: RoutingParams): void;
  };
  
  // 向量操作
  vector: {
    generate(text: string): Promise<number[]>;
    batchGenerate(texts: string[]): Promise<number[][]>;
    search(vector: number[], topK: number): Promise<VectorSearchResult[]>;
  };
  
  // 状态管理
  status: {
    report(status: NodeStatus): Promise<void>;
    get(did: string): Promise<NodeStatus>;
    subscribe(dids: string[], callback: StatusCallback): Promise<void>;
  };
}
```

### 7.2 错误码

| 错误码 | 描述 | 处理建议 |
|--------|------|----------|
| `DS001` | Schema 验证失败 | 检查 Schema 格式 |
| `DS002` | DID 未注册 | 先注册能力 |
| `DS003` | 向量生成失败 | 检查 Embedding 模型 |
| `DS004` | 无匹配候选 | 降低阈值或扩展意图描述 |
| `DS005` | DHT 查询超时 | 检查网络连接 |
| `DS006` | 节点离线 | 选择其他候选 |
| `DS007` | 负载过高 | 等待或选择其他节点 |
| `DS008` | 余额不足 | 充值或选择低成本服务 |

---

## 8. 性能考量

### 8.1 性能目标

| 指标 | 目标值 | 测量方法 |
|------|--------|----------|
| **向量生成延迟** | < 50ms | 从文本输入到向量输出 |
| **ANN 搜索延迟** | < 30ms | 从向量输入到候选输出 |
| **路由决策延迟** | < 20ms | 从候选到最终选择 |
| **总路由延迟** | < 100ms | 从意图到目标节点 |
| **DHT 同步延迟** | < 5s | 跨 Supernode 数据同步 |

### 8.2 优化策略

#### 8.2.1 向量缓存

```python
class VectorCache:
    """向量缓存，避免重复计算"""
    
    def __init__(self, max_size: int = 10000):
        self.cache = {}
        self.max_size = max_size
    
    def get_or_compute(self, text: str, vectorizer: SemanticVectorizer) -> np.ndarray:
        # 检查缓存
        cache_key = self._hash_text(text)
        if cache_key in self.cache:
            return self.cache[cache_key]
        
        # 计算向量
        vector = vectorizer.vectorize_intent(text)
        
        # 存入缓存
        if len(self.cache) >= self.max_size:
            self._evict_oldest()
        self.cache[cache_key] = vector
        
        return vector
```

#### 8.2.2 索引优化

```yaml
# HNSW 索引参数优化
hnsw_params:
  M: 16              # 每层连接数（平衡精度与内存）
  efConstruction: 200  # 构建时搜索宽度
  efSearch: 50       # 搜索时搜索宽度
  
# 对于 100 节点网络：
# - 内存占用：约 100 * 384 * 4 * 16 ≈ 2.5MB
# - 搜索延迟：约 10-30ms
```

#### 8.2.3 批量处理

```python
async def batch_route(intents: list[str]) -> list[RoutingResult]:
    """批量路由，提高吞吐量"""
    
    # 1. 批量向量化
    vectors = await vectorizer.batch_vectorize(intents)
    
    # 2. 批量 ANN 搜索
    all_candidates = await dht_client.batch_search(vectors, top_k=10)
    
    # 3. 批量权重计算
    results = []
    for i, candidates in enumerate(all_candidates):
        result = routing_engine.select_best(candidates)
        results.append(result)
    
    return results
```

---

## 9. 相关文档

### 上层架构

- [整体架构设计](./ARCHITECTURE.md) - 四层架构总览
- [身份与零信任网络层](./IDENTITY_LAYER.md) - Layer 1 设计

### 下层依赖

- [传输与协商协议层](./TRANSPORT_LAYER.md) - Layer 3 设计
- [资源管理与微交易层](./ECONOMY_LAYER.md) - Layer 4 设计

### 相关规范

- [协议规范](./PROTOCOL_SPEC.md) - 发现层协议定义
- [API 参考](./API_REFERENCE.md) - 发现层 API 定义

### 参考资料

- [OpenAPI 3.1 Specification](https://spec.openapis.org/oas/v3.1.0)
- [Model Context Protocol](https://modelcontextprotocol.io/)
- [FAISS: Efficient Similarity Search](https://faiss.ai/)
- [HNSW Algorithm](https://arxiv.org/abs/1603.09320)