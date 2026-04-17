# Nexa-net 技术报告撰写计划

## 概述

在 `docs/tech_report/` 目录中，使用 ACM SIGCOMM 论文模板（`acmart` sigconf 格式）撰写一份不限页数的技术报告，详细阐述 Nexa-net 去中心化 Agent 网络系统的设计、实现、测试和分析。

## 模板选择

**ACM SIGCOMM** — 网络系统领域顶会，使用 `\documentclass[sigconf]{acmart}` 双栏格式。

模板来源: GitHub `conference-websites/acmart-sigproc-template` 或 CTAN `acmart` 包。
由于 `acmart.cls` 已包含在 TeX Live 中，只需创建 `main.tex` + `references.bib` 即可编译。

## 论文结构

### 1. Abstract (~200 words)
- Nexa-net 是什么：去中心化 Agent 间语义通信的 Sidecar Proxy 网络
- 核心问题：现有 Agent 框架缺乏语义路由、经济激励、零信任安全
- 解决方案：四层架构 + Sidecar Proxy + 语义发现 + 状态通道微交易
- 关键结果：路由延迟 31µs、通道 TPS 9.9M、485 测试全部通过

### 2. Introduction (Motivation) (~2 pages)
- **问题背景**: LLM/Agent 爆发式增长 → Agent 间通信需求激增
- **现有痛点**:
  - Agent 通信依赖中心化平台（OpenAI API、LangChain Hub）→ 单点故障
  - 缺乏语义级路由 → 精确意图匹配困难
  - 无经济激励 → 服务提供者无动力
  - 安全模型薄弱 → API key 脆弱
- **Nexa-net 定位**: 非侵入式 Sidecar Proxy → Agent 无需修改代码即可接入去中心化网络
- **核心贡献**:
  1. 四层去中心化 Agent 网络架构
  2. 基于 HNSW 的语义路由算法
  3. Ed25519 + 状态通道的零信任微交易机制
  4. 完整 Rust 实现 + 485 测试 + 45 benchmarks

### 3. Background & Related Work (~2-3 pages)
- **Agent 框架**: AutoGen, CrewAI, LangChain, OpenAI Swarm → 中心化
- **P2P 网络**: BitTorrent, IPFS, Kademlia DHT → 无语义路由
- **Service Discovery**: Consul, etcd, DNS SRV → 无语义匹配
- **Micro-payment**: Bitcoin Lightning, Raiden → 链上结算太慢
- **Semantic Search**: FAISS, HNSW, word2vec → 无 Agent 能力路由
- **零信任**: SPIFFE/SPIRE, BeyondCorp → 无 Agent 身份模型
- **Sidecar**: Envoy, Istio → 无 Agent 经济激励
- **DID/VC**: W3C DID, Verifiable Credentials → 无 Agent 间协商

### 4. System Architecture Overview (~2 pages)
- 四层架构模型图（Mermaid → TikZ）
- Sidecar Proxy 模式设计哲学
- 组件交互流程图
- 数据流: Agent → Proxy → Supernode → 远程 Agent

### 5. Layer 1: Identity & Zero-Trust (~3 pages)
- Nexa-DID 标识符设计（W3C DID 兼容）
- Ed25519/X25519 密钥管理 + zeroize
- Verifiable Credentials 签发与验证
- DID Document 结构
- mTLS 互认证设计
- Trust Anchor 信任锚

### 6. Layer 2: Semantic Discovery & Routing (~3-4 pages)
- Capability Schema 能力描述
- Mock/ONNX 语义向量化
- HNSW 向量索引 + cosine distance SIMD 优化
- Kademlia DHT 路由表
- Semantic Router 多因子评分公式
- 路由权重: similarity + quality + cost + load + latency

### 7. Layer 3: Transport & Protocol (~2-3 pages)
- 12 字节帧协议格式
- 多路复用流状态机 (Idle→Open→HalfClosed→Closed)
- RPC 4 种模式 (Unary/Server-Stream/Client-Stream/Bidi)
- SYN-NEXA/ACK-SCHEMA 协议协商
- 序列化引擎 (JSON + LZ4/Zstd/Gzip 压缩)
- 错误处理 + 指数退避重试

### 8. Layer 4: Economy & Micro-Transaction (~3 pages)
- 状态通道生命周期 (Open→Active→Closing→Closed)
- MicroReceipt 收据 + 双签名 + SHA-256 哈希链
- BudgetController 多级预算控制
- SettlementEngine 结算引擎
- NexaToken token 引擎
- 余额不变量: total = balance_a + balance_b

### 9. Proxy & API Layer (~2 pages)
- ProxyState 组件容器
- REST API 7 端点设计
- gRPC Health Service
- NexaClient SDK Builder 模式

### 10. Security Layer (~2 pages)
- AES-256-GCM 加密存储
- RateLimiter DashMap 无锁限速
- AuditLogger 审计日志
- KeyRotator 密钥轮换
- SecurityManager 经带协调器

### 11. Storage Layer (~1-2 pages)
- MemoryStore DashMap 实现
- RocksDB/PostgreSQL/Redis feature-gated 后端
- Storage trait 统一接口

### 12. Testing & Evaluation (~4-5 pages)
- **测试体系**: 485 tests (433 unit + 5 HTTP E2E + ...)
- **5 个 E2E 场景**: 双 Agent 通信、多 Agent 社区、故障恢复、经济闭环、安全验证
- **Property-based testing**: proptest 1000 次迭代
- **HTTP E2E 测试**: TestProxy 随机端口 + graceful shutdown
- **数据表格**: 各模块测试覆盖率统计

### 13. Performance Analysis (~4-5 pages + 图表)
- **45 个 Criterion benchmarks** 覆盖 8 模块
- 关键性能数据:
  - 路由延迟 31µs (目标 100ms, 3200x)
  - 通道 TPS 9.9M (目标 10K, 990x)
  - 序列化 5.6M ops/s (目标 100K, 56x)
  - REST API: health 1.2ms, discover 1.4ms
- **Python 图表** (6-8 个):
  - Identity: keypair/sign/verify 延迟柱状图
  - Discovery: HNSW search 延迟 vs 索引大小折线图
  - Transport: 压缩算法吞吐量对比柱状图
  - Economy: 通道 TPS vs 操作数折线图
  - Security: AES-GCM 加密延迟 vs 数据大小折线图
  - REST API: 端点延迟对比柱状图
  - Overall: 性能目标 vs 实际雷达图
- **优化措施**: DashMap/SIMD/Pre-allocation

### 14. Discussion & Limitations (~1-2 pages)
- **优势**: 非侵入式、语义路由、零信任、微交易
- **局限**:
  - 无真实网络传输（当前仅内存级）
  - gRPC 服务仅 health check
  - Proxy client 为 placeholder
  - ONNX embedding 需下载模型
  - 未实现 mTLS 实际握手
  - 未实现 supernode 网络发现
- **未来方向**: 真实网络层、ONNX 生产模型、多语言 SDK、跨链结算

### 15. Conclusion (~1 page)
- 总结核心贡献
- Nexa-net 开源愿景

### 16. References (50+ entries)
覆盖: Agent 框架、P2P 网络、DHT、语义搜索、微支付、零信任、DID/VC、Rust 系统、网络协议、分布式系统

### 17. Appendix
- A: 帧协议详细格式
- B: 路由评分公式推导
- C: 收据哈希链算法
- D: 完整测试列表摘要
- E: 代码仓库结构

---

## 文件结构

```
docs/tech_report/
├── main.tex                  # 主论文文件 (acmart sigconf)
├── references.bib            # 50+ 引用文献
├── figures/                  # Python 生成的图表
│   ├── gen_identity_bench.py     # Identity 性能柱状图
│   ├── gen_discovery_bench.py    # HNSW search 折线图
│   ├── gen_transport_bench.py    # 压缩吞吐量对比图
│   ├── gen_economy_bench.py      # 通道 TPS 折线图
│   ├── gen_security_bench.py     # AES-GCM 延迟图
│   ├── gen_api_bench.py          # REST API 延迟对比图
│   ├── gen_overall_radar.py      # 性能目标 vs 实际雷达图
│   └── gen_test_coverage.py      # 测试覆盖率饼图
│   ├── identity_bench.pdf
│   ├── discovery_bench.pdf
│   ├── transport_bench.pdf
│   ├── economy_bench.pdf
│   ├── security_bench.pdf
│   ├── api_bench.pdf
│   ├── overall_radar.pdf
│   └── test_coverage.pdf
├── acmart.cls                # ACM 模板类文件 (从 TeX Live 获取)
├── ACM-Reference-Format.bst  # ACM 引用格式
└── Makefile                  # 编译脚本
```

## 执行步骤

1. **下载模板**: 从 CTAN/GitHub 获取 `acmart.cls` + `ACM-Reference-Format.bst` + 相关字体文件
2. **创建 main.tex**: 按 acmart sigconf 格式编写完整论文
3. **创建 references.bib**: 50+ 引用文献条目
4. **创建 Python 图表脚本**: 8 个 `figures/*.py` 文件
5. **运行 Python 脚本生成图表**: `python figures/*.py` → PDF
6. **编译 LaTeX**: `pdflatex main && bibtex main && pdflatex main && pdflatex main`
7. **验证 PDF 输出**: 确认图表、引用、排版正确

## Python 图表依赖
- matplotlib + numpy (标准科学绘图)
- 输出格式: PDF (LaTeX 可直接 includegraphics)
- 数据来源: `docs/PERFORMANCE.md` 中的实际 benchmark 数据