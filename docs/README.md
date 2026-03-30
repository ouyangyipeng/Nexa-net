# Nexa-net 文档中心

> **版本:** v1.0.0-draft | **最后更新:** 2026-03-30

## 📖 项目概述

**Nexa-net** 是一个专为自治智能体（Autonomous Agents）设计的去中心化、非侵入式机器间通讯（M2M）基础设施。

### 核心特性

- 🔄 **非侵入式架构** - 基于 Sidecar Proxy 模式，现有 Agent 无需重写核心逻辑
- 🔐 **零信任安全** - 基于密码学的去中心化身份（DID）与相互认证
- 🎯 **语义路由** - 基于向量语义的能力发现，替代传统 DNS
- ⚡ **高性能传输** - 二进制 RPC 协议，体积缩小 60%-80%
- 💰 **原生微交易** - 内置 Layer 2 状态通道，支持高频 M2M 经济

### 与传统方案的对比

| 特性 | HTTP/REST API | Nexa-net |
|------|---------------|----------|
| 数据格式 | 文本 JSON | 二进制 Protobuf/FlatBuffers |
| 服务发现 | DNS + URL | 语义向量路由 |
| 身份认证 | API Key / OAuth | DID + mTLS |
| 支付结算 | 外部服务 | 内置状态通道 |
| Agent 友好度 | 需要适配 | 开箱即用 |

---

## 📚 文档导航

### 🚀 快速开始

| 文档 | 描述 | 适用读者 |
|------|------|----------|
| [开发者接入指南](./DEVELOPER_GUIDE.md) | 5 分钟快速接入 Nexa-net | Agent 开发者 |
| [部署运维指南](./DEPLOYMENT.md) | 部署拓扑与运维实践 | DevOps 工程师 |

### 🏗️ 架构设计

| 文档 | 描述 | 适用读者 |
|------|------|----------|
| [整体架构设计](./ARCHITECTURE.md) | 四层架构总览与设计哲学 | 架构师、技术负责人 |
| [身份与零信任层](./IDENTITY_LAYER.md) | DID、mTLS、可验证凭证 | 安全工程师 |
| [语义发现与路由层](./DISCOVERY_LAYER.md) | 能力 Schema、语义路由算法 | 后端工程师 |
| [传输与协议层](./TRANSPORT_LAYER.md) | 协议协商、流式 RPC | 网络工程师 |
| [资源与经济层](./ECONOMY_LAYER.md) | 状态通道、微交易引擎 | 区块链工程师 |

### 📋 技术规范

| 文档 | 描述 | 适用读者 |
|------|------|----------|
| [协议规范](./PROTOCOL_SPEC.md) | 二进制协议格式、消息类型、错误码 | 协议开发者 |
| [API 参考](./API_REFERENCE.md) | gRPC/REST API 定义、SDK 接口 | 应用开发者 |
| [安全设计](./SECURITY.md) | 威胁模型、安全机制、审计清单 | 安全审计员 |

### 📖 参考资料

| 文档 | 描述 | 适用读者 |
|------|------|----------|
| [术语表](./GLOSSARY.md) | 术语定义与缩写索引 | 所有读者 |
| [项目路线图](./ROADMAP.md) | 里程碑、技术决策记录 | 项目管理者 |

---

## 🗺️ 文档结构图

```
docs/
├── README.md              # 本文档 - 文档导航入口
├── ARCHITECTURE.md        # 整体架构设计概览
├── IDENTITY_LAYER.md      # 第一层：身份与零信任网络
├── DISCOVERY_LAYER.md     # 第二层：语义发现与能力路由
├── TRANSPORT_LAYER.md     # 第三层：传输与协商协议
├── ECONOMY_LAYER.md       # 第四层：资源管理与微交易
├── PROTOCOL_SPEC.md       # 协议规范详细定义
├── API_REFERENCE.md       # API 接口规范
├── SECURITY.md            # 安全设计规范
├── DEPLOYMENT.md          # 部署拓扑与运维指南
├── DEVELOPER_GUIDE.md     # 开发者接入指南
├── GLOSSARY.md            # 术语表
├── ROADMAP.md             # 项目路线图与决策记录
└── INIT_DESIGN.md         # 初始设计草案（历史参考）
```

---

## 🔗 快速链接

### 按角色导航

**🤖 Agent 开发者**
1. 阅读 [开发者接入指南](./DEVELOPER_GUIDE.md)
2. 参考 [API 文档](./API_REFERENCE.md)
3. 查阅 [术语表](./GLOSSARY.md)

**🏗️ 架构师**
1. 阅读 [整体架构设计](./ARCHITECTURE.md)
2. 深入各层设计文档
3. 参考 [协议规范](./PROTOCOL_SPEC.md)

**🔐 安全工程师**
1. 阅读 [安全设计规范](./SECURITY.md)
2. 参考 [身份层设计](./IDENTITY_LAYER.md)
3. 审计 [协议规范](./PROTOCOL_SPEC.md)

**🚀 DevOps 工程师**
1. 阅读 [部署运维指南](./DEPLOYMENT.md)
2. 参考 [架构设计](./ARCHITECTURE.md)
3. 配置监控与告警

---

## 📝 文档约定

### 状态标记

- ✅ 已完成并审核
- 🚧 草稿阶段
- 📋 计划中
- ⚠️ 需要更新

### 图表说明

文档中使用 Mermaid 语法绘制架构图和流程图，确保在支持 Mermaid 的 Markdown 渲染器中查看。

### 版本兼容性

所有文档遵循语义化版本控制。当前版本为 `v1.0.0-draft`，表示设计阶段。正式发布后将更新为 `v1.0.0`。

---

## 🤝 贡献指南

如需修改或补充文档，请遵循以下原则：

1. **单一职责** - 每个文档聚焦一个主题
2. **交叉引用** - 使用相对链接引用相关文档
3. **同步更新** - 修改设计时同步更新 ROADMAP.md
4. **术语一致** - 使用 GLOSSARY.md 中定义的标准术语

---

## 📜 许可证

本文档采用 [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/) 许可证。