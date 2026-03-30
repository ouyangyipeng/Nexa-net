# Nexa-net 技术规格与架构手册 (v1.0-Draft)

## 0. 摘要 (Abstract)
Nexa-net 是一个专为自治智能体（Autonomous Agents）设计的去中心化、非侵入式机器间通讯（M2M）基础设施。有别于基于人类消费设计的 HTTP/HTML 互联网，Nexa-net 摒弃了所有视觉渲染与非结构化数据传输层，基于**边车代理（Sidecar Proxy）**模式，实现跨异构 Agent 框架的开箱即用。其核心支柱包括：去中心化身份（DID）、基于向量语义的能力路由、强类型 RPC 协议流、以及原生的微交易结算引擎。

## 1. 核心架构哲学：Nexa Sidecar Proxy
Nexa-net 采用非侵入式架构。现有的 Agent（无论是基于 LangChain、AutoGen 还是自定义的裸大模型脚本）**不需要重写核心逻辑**。



每个 Agent 节点在本地运行一个轻量级的守护进程——**Nexa-Proxy**。
* **Data Plane（数据平面）：** 负责拦截本地 Agent 发出的工具调用（Tool Calls），将其转换为高密度的二进制 RPC 协议（如基于 HTTP/2 的 gRPC 或 Cap'n Proto）并路由到目标 Agent。
* **Control Plane（控制平面）：** 负责节点发现、能力注册、密钥协商和微交易状态同步。本地 Agent 只需将 Nexa-Proxy 视为一个名为 `Nexa_Network_Interface` 的本地函数/工具即可。

## 2. 第一层：身份与零信任网络 (Identity & Zero-Trust Layer)
在 100% 由机器组成的网络中，传统的密码和 IP 白名单失效。Nexa-net 采用基于密码学的绝对零信任架构。

### 2.1 去中心化机器标识 (Nexa-DID)
每个加入网络的 Agent 在初始化时生成一对非对称密钥（推荐使用 Ed25519 或 Secp256k1）。其身份标识（Nexa-ID）即为公钥的哈希值。
* 格式示例：`did:nexa:1a2b3c4d5e6f7g8h9i0j...`
* **相互认证 (mTLS)：** 当 Agent A 的代理尝试连接 Agent B 的代理时，必须完成双向 TLS 握手。网络层自动拒绝任何未签名的连接请求。

### 2.2 权限控制与可验证凭证 (Verifiable Credentials, VC)
权限不依赖中心化数据库，而是通过密码学签名的凭证传递。如果 Agent A 拥有调用 Agent B “高级数据清洗引擎”的权限，A 会在握手时出示由网络信任锚（Trust Anchor）签名的 VC。
权限验证的计算复杂度极低，完全在本地 Proxy 完成。

## 3. 第二层：语义发现与能力路由 (Semantic Discovery & Capability Routing)
这是 Nexa-net 替代 DNS（域名系统）的核心。Agent 不通过 URL 寻找目标，而是通过**“意图（Intent）”**寻找。

### 3.1 能力清单 (Capability Schema)
当一个 Agent 启动并在 Nexa-net 注册时，它必须提交一份严格的机器可读清单，基于扩展的 OpenAPI 3.1 或 Model Context Protocol (MCP) 规范。
包含：
* **Endpoint:** 提供什么服务（如 `audio_to_text`）。
* **Input/Output Schema:** 严格的 JSON Schema 定义。
* **Cost/Rate Limit:** 每次调用的 Token 消耗或微交易报价。

### 3.2 向量化语义路由算法
为了在 100 个甚至上万个 Agent 的社区中快速寻址，Nexa-net 维护一个分布式的语义哈希表（Semantic DHT）。节点的能力描述会被预先通过轻量级 Embedding 模型（如 `all-MiniLM-L6-v2`）转换为高维向量。

当路由请求到达时，Nexa-Proxy 将任务需求向量化为 $V_{req}$，并与网络中已注册的节点能力向量 $V_{node}$ 计算余弦相似度（Cosine Similarity）：
$$\text{Similarity} = \frac{V_{req} \cdot V_{node}}{||V_{req}|| \times ||V_{node}||}$$

系统设置一个动态阈值 $\tau$。只有当 $\text{Similarity} > \tau$ 时，节点才会被选为候选目标。随后，代理系统会结合节点的当前负载、网络延迟和报价（Cost）计算最终的路由权重 $W$：
$$W = \alpha \cdot \text{Similarity} - \beta \cdot \text{Latency} - \gamma \cdot \text{Cost}$$
（其中 $\alpha, \beta, \gamma$ 为可调的权重系数）。最优权重的节点将获得该子任务的执行权。



## 4. 第三层：传输与协商协议 (Transport & Contract Layer)
确定目标后，系统进入高密度的数据传输阶段。抛弃 HTTP 文本传输，全面转向二进制。

### 4.1 动态协议协商 (Dynamic Handshake)
双方的 Nexa-Proxy 在建立连接的最初 50 毫秒内完成协商。
1.  **SYN-NEXA:** 携带调用方的意图 Hash 和最高预算。
2.  **ACK-SCHEMA:** 响应方返回当前工具的压缩二进制 Schema。
3.  **EXEC:** 调用方按照 Schema 严格组装参数并发送。

### 4.2 结构化流式 RPC (Structured Streaming RPC)
针对 Agent 之间经常需要处理大规模上下文（如传递 100K token 的文本、传递图像矩阵）的需求，Nexa-net 采用流式多路复用传输。
数据在传输前通过 Protobuf 或 FlatBuffers 进行序列化，相比传统的 JSON 文本，体积缩小 60%-80%，序列化/反序列化速度提升一个数量级。这从根本上解决了图片中提到的“白白烧掉大量 token”且“速度提不上去”的瓶颈。

## 5. 第四层：资源管理与机器微交易 (M2M Economy Layer)
在去中心化网络中，没有免费的算力。如果 100 个 Agent 互相调用，必须有内置的经济护栏，防止“死循环调用”耗尽物理资源。

### 5.1 Nexa-Token 与状态通道 (State Channels)
Nexa-net 协议栈内置了极其轻量的 Layer 2 状态通道协议。Agent 之间不需要每次调用都去区块链主网或银行接口排队结算，这会导致不可接受的延迟。
* **通道建立：** Agent A 和 B 预先锁定一部分信用额度（或虚拟 Token）开启状态通道。
* **高频微支付：** 每发生一次 API 调用，双方 Nexa-Proxy 在本地默默更新并签名一张“微交易收据”（Micro-Receipt）。
假设单次调用成本为 $c$，共调用 $n$ 次，最终状态结算为：
$$\text{Total Cost} = \sum_{i=1}^{n} c_i \quad \text{subject to} \quad \text{Total Cost} \le \text{Locked Quota}$$
* 当且仅当通道关闭时，最终的净余额才会在全局账本（或中央统计服务器）上结算。这使得 10 万 TPS 的高频微交易成为可能，开销几乎为零。



## 6. 拓扑与部署：100-Agent 社区实例 (Deployment Topology)
对于您设想的 100 个 Agent 的特定社区，Nexa-net 建议采用**带超级节点的联邦制拓扑（Federated Topology with Supernodes）**，而非绝对的纯 P2P，以换取更高的寻址效率。

1.  **Supernode (Registry/Relay):** 部署 3-5 个高可用的中心化节点。它们不执行具体的大模型推理任务，只负责运行 Semantic DHT（语义路由表）和 STUN/TURN 穿透服务。
2.  **Edge Nodes (Agent Proxy):** 剩余的 90 多个 Agent 散布在不同的物理机器或容器中。它们通过本地的 Nexa-Proxy 连接到 Supernode，保持长连接。
3.  **P2P Fallback (可选):** 当两个 Agent 在同一个内网或能够完成 NAT 穿透时，Nexa-Proxy 会自动将流量从 Supernode 中继切换为 P2P 直连（WebRTC 数据通道协议），将网络延迟降至最低。

## 7. 给开发者的接入指南 (Developer Experience)
Nexa-net 的核心竞争力在于“无缝”。

* **对于人类开发者：** 下载 `nexa-proxy` 二进制文件并在后台运行。
* **在 Agent 代码中（以 Python 为例）：**
    不需要引入复杂的网络库，只需引入一个标准工具。

```python
# 传统的 Agent 需要手写 requests.post 去调别人的 API
# Nexa-net 的 Agent 只需调用本地网络接口

nexa_tool = {
    "name": "nexa_network_call",
    "description": "向 Nexa 网络广播意图并获取结构化结果。当你的本地工具无法解决问题时调用此工具。",
    "parameters": {
        "intent": "需要将以下英文 PDF 翻译为中文并提取关键指标",
        "data": "<base64_pdf_data>",
        "max_budget": 50 # 愿意支付的微交易代币上限
    }
}
# 将此 tool 喂给你的 LLM 即可。底层所有复杂的握手、路由、加密，全部由宿主机上的 Nexa-Proxy 劫持并处理。
```