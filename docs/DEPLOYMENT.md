# Nexa-net 部署拓扑与运维指南

> **版本:** v1.0.0-draft | **最后更新:** 2026-03-30

## 目录

- [1. 部署概述](#1-部署概述)
- [2. 网络拓扑设计](#2-网络拓扑设计)
- [3. Supernode 部署](#3-supernode-部署)
- [4. Nexa-Proxy 部署](#4-nexa-proxy-部署)
- [5. 容器化部署](#5-容器化部署)
- [6. 监控与告警](#6-监控与告警)
- [7. 日志管理](#7-日志管理)
- [8. 故障排查](#8-故障排查)
- [9. 运维最佳实践](#9-运维最佳实践)
- [10. 相关文档](#10-相关文档)

---

## 1. 部署概述

### 1.1 部署架构总览

Nexa-net 采用**带超级节点的联邦制拓扑**，而非绝对的纯 P2P，以换取更高的寻址效率：

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         Nexa-net Deployment Overview                    │
│                                                                         │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                      Supernode Cluster                          │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐              │   │
│  │  │ Supernode 1 │  │ Supernode 2 │  │ Supernode 3 │              │   │
│  │  │ (Primary)   │  │ (Secondary) │  │ (Secondary) │              │   │
│  │  │             │  │             │  │             │              │   │
│  │  │ - Registry  │  │ - Registry  │  │ - Registry  │              │   │
│  │  │ - Relay     │  │ - Relay     │  │ - Relay     │              │   │
│  │  │ - STUN/TURN │  │ - STUN/TURN │  │ - STUN/TURN │              │   │
│  │  │ - DHT       │  │ - DHT       │  │ - DHT       │              │   │
│  │  └─────────────┘  └─────────────┘  └─────────────┘              │   │
│  │         │               │               │                        │   │
│  │         └───────────────┴───────────────┘                        │   │
│  │                    Replication & Sync                            │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                              │                                          │
│                              │ Long Connections                         │
│                              ▼                                          │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                        Edge Nodes                                │   │
│  │                                                                  │   │
│  │  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐  │   │
│  │  │   Agent Host 1  │  │   Agent Host 2  │  │   Agent Host 3  │  │   │
│  │  │                 │  │                 │  │                 │  │   │
│  │  │ ┌─────────────┐ │  │ ┌─────────────┐ │  │ ┌─────────────┐ │  │   │
│  │  │ │ Nexa-Proxy  │ │  │ │ Nexa-Proxy  │ │  │ │ Nexa-Proxy  │ │  │   │
│  │  │ │             │ │  │ │             │ │  │ │             │ │  │   │
│  │  │ └─────────────┘ │  │ └─────────────┘ │  │ └─────────────┘ │  │   │
│  │  │ ┌─────────────┐ │  │ ┌─────────────┐ │  │ ┌─────────────┐ │  │   │
│  │  │ │ Agent A     │ │  │ │ Agent B     │ │  │ │ Agent C     │ │  │   │
│  │  │ │ (LangChain) │ │  │ │ (AutoGen)   │ │  │ │ (Custom)    │ │  │   │
│  │  │ └─────────────┘ │  │ └─────────────┘ │  │ └─────────────┘ │  │   │
│  │  └─────────────────┘  └─────────────────┘  └─────────────────┘  │   │
│  │                                                                  │   │
│  │  ─────────────────── P2P Mesh (WebRTC) ───────────────────────  │   │
│  │                                                                  │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

### 1.2 部署规模建议

| 场景 | Agent 数量 | Supernode 数量 | 建议配置 |
|------|------------|----------------|----------|
| **小型社区** | 10-50 | 1 | 单 Supernode，低配 |
| **中型社区** | 50-200 | 3 | 3 Supernode，中配 |
| **大型社区** | 200-1000 | 5 | 5 Supernode，高配 |
| **企业级** | 1000+ | 7+ | 多区域部署 |

### 1.3 部署组件清单

| 组件 | 部署位置 | 数量 | 资源需求 |
|------|----------|------|----------|
| **Supernode** | 高可用服务器 | 3-5 | CPU: 4核+, RAM: 16GB+, SSD: 100GB+ |
| **Nexa-Proxy** | Agent 同机 | N | CPU: 1核, RAM: 2GB, SSD: 10GB |
| **Ledger Node** | 可选 | 1-3 | CPU: 2核+, RAM: 8GB+, SSD: 50GB+ |
| **Monitoring** | 管理服务器 | 1 | CPU: 2核, RAM: 4GB, SSD: 50GB |

---

## 2. 网络拓扑设计

### 2.1 联邦制拓扑详解

#### 2.1.1 Supernode 角色

```
┌─────────────────────────────────────────────────────────────┐
│                    Supernode Roles                          │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Registry Service                        │   │
│  │  - 维护 Semantic DHT                                 │   │
│  │  - 存储能力索引                                      │   │
│  │  - 处理注册/注销请求                                  │   │
│  │  - 提供语义路由查询                                  │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Relay Service                           │   │
│  │  - 中继 Agent 间消息                                 │   │
│  │  - NAT 穿透辅助                                      │   │
│  │  - 流量统计                                          │   │
│  │  - 负载均衡                                          │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              STUN/TURN Service                       │   │
│  │  - NAT 类型检测                                      │   │
│  │  - 公网 IP 发现                                      │   │
│  │  - TURN 中继（无法直连时）                            │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Coordination Service                    │   │
│  │  - 节点健康监控                                      │   │
│  │  - 网络拓扑管理                                      │   │
│  │  - 全局配置分发                                      │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

#### 2.1.2 Edge Node 角色

```
┌─────────────────────────────────────────────────────────────┐
│                    Edge Node Roles                          │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Nexa-Proxy                              │   │
│  │  - 本地 Agent 代理                                   │   │
│  │  - 协议转换                                          │   │
│  │  - 加密/解密                                         │   │
│  │  - 路由决策                                          │   │
│  │  - 微交易管理                                        │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Agent Runtime                           │   │
│  │  - 业务逻辑执行                                      │   │
│  │  - 大模型推理                                        │   │
│  │  - 工具调用                                          │   │
│  │  - 状态管理                                          │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Local Services                          │   │
│  │  - 本地能力注册                                      │   │
│  │  - 本地缓存                                          │   │
│  │  - 本地日志                                          │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### 2.2 网络连接模式

#### 2.2.1 连接类型

| 连接类型 | 描述 | 适用场景 |
|----------|------|----------|
| **Supernode-Edge** | Edge Node 连接到 Supernode | 注册、路由查询、中继 |
| **Edge-Edge (P2P)** | Edge Node 之间直连 | 高频数据传输 |
| **Edge-Edge (Relayed)** | 通过 Supernode 中继 | NAT 无法穿透 |
| **Supernode-Supernode** | Supernode 之间同步 | 数据复制、状态同步 |

#### 2.2.2 P2P 直连优先

```
┌─────────────────────────────────────────────────────────────┐
│                    P2P Connection Strategy                  │
│                                                             │
│  优先级 1: P2P 直连 (WebRTC Data Channel)                   │
│  ┌─────────────┐                    ┌─────────────┐         │
│  │ Nexa-Proxy A│───────────────────▶│ Nexa-Proxy B│         │
│  └─────────────┘                    └─────────────┘         │
│  条件: NAT 穿透成功                                          │
│  延迟: < 10ms                                                │
│                                                             │
│  优先级 2: Supernode 中继                                    │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐         │
│  │ Nexa-Proxy A│──▶│  Supernode  │──▶│ Nexa-Proxy B│         │
│  └─────────────┘  └─────────────┘  └─────────────┘         │
│  条件: NAT 穿透失败                                          │
│  延迟: 50-100ms                                              │
│                                                             │
│  自动切换逻辑:                                               │
│  1. 尝试 NAT 穿透                                            │
│  2. 穿透成功 → P2P 直连                                      │
│  3. 穿透失败 → Supernode 中继                                │
│  4. 定期重试 P2P 直连                                        │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### 2.3 多区域部署

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    Multi-Region Deployment                              │
│                                                                         │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                      Region A (Asia)                            │   │
│  │  ┌─────────────┐                                                │   │
│  │  │ Supernode A │                                                │   │
│  │  │ (Primary)   │                                                │   │
│  │  └─────────────┘                                                │   │
│  │         │                                                        │   │
│  │         │                                                        │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐              │   │
│  │  │ Agent Host 1│  │ Agent Host 2│  │ Agent Host 3│              │   │
│  │  └─────────────┘  └─────────────┘  └─────────────┘              │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                              │                                          │
│                              │ Cross-Region Link                        │
│                              ▼                                          │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                      Region B (Europe)                          │   │
│  │  ┌─────────────┐                                                │   │
│  │  │ Supernode B │                                                │   │
│  │  │ (Primary)   │                                                │   │
│  │  └─────────────┘                                                │   │
│  │         │                                                        │   │
│  │         │                                                        │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐              │   │
│  │  │ Agent Host 4│  │ Agent Host 5│  │ Agent Host 6│              │   │
│  │  └─────────────┘  └─────────────┘  └─────────────┘              │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                              │                                          │
│                              │ Cross-Region Link                        │
│                              ▼                                          │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                      Region C (Americas)                        │   │
│  │  ┌─────────────┐                                                │   │
│  │  │ Supernode C │                                                │   │
│  │  │ (Primary)   │                                                │   │
│  │  └─────────────┘                                                │   │
│  │         │                                                        │   │
│  │         │                                                        │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐              │   │
│  │  │ Agent Host 7│  │ Agent Host 8│  │ Agent Host 9│              │   │
│  │  └─────────────┘  └─────────────┘  └─────────────┘              │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                                                         │
│  跨区域路由策略:                                                         │
│  - 优先本区域 Supernode                                                  │
│  - 跨区域调用通过区域间链路                                               │
│  - 区域间数据异步同步                                                     │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 3. Supernode 部署

### 3.1 系统要求

#### 3.1.1 硬件要求

```yaml
# supernode_hardware.yaml
minimum:
  cpu: 4 cores
  memory: 16 GB
  storage: 100 GB SSD
  network: 1 Gbps

recommended:
  cpu: 8 cores
  memory: 32 GB
  storage: 500 GB NVMe SSD
  network: 10 Gbps

high_availability:
  cpu: 16 cores
  memory: 64 GB
  storage: 1 TB NVMe SSD
  network: 25 Gbps
```

#### 3.1.2 软件要求

```yaml
# supernode_software.yaml
os:
  - Ubuntu 22.04 LTS
  - Debian 12
  - CentOS 9

dependencies:
  - Docker 24.0+
  - Docker Compose 2.20+
  - Python 3.11+
  - Node.js 20+
  - Rust 1.75+

network:
  - Open ports: 443, 7070, 3478 (STUN), 5349 (TURN)
  - TLS certificate
  - Domain name
```

### 3.2 部署步骤

#### 3.2.1 准备工作

```bash
# 1. 安装依赖
sudo apt update
sudo apt install -y docker.io docker-compose-plugin python3.11 nodejs rustc cargo

# 2. 创建目录
sudo mkdir -p /opt/nexa/supernode
sudo mkdir -p /var/lib/nexa/data
sudo mkdir -p /var/lib/nexa/logs

# 3. 获取 TLS 证书
sudo certbot certonly --standalone -d supernode.your-domain.com

# 4. 设置权限
sudo chown -R nexa:nexa /opt/nexa /var/lib/nexa
```

#### 3.2.2 配置文件

```yaml
# /opt/nexa/supernode/config.yaml
supernode:
  id: "supernode-1"
  region: "asia-east"
  role: "primary"  # primary or secondary
  
network:
  listen_address: "0.0.0.0"
  public_address: "supernode.your-domain.com"
  ports:
    https: 443
    nexa: 7070
    stun: 3478
    turn: 5349
    
tls:
  cert_file: "/etc/letsencrypt/live/supernode.your-domain.com/fullchain.pem"
  key_file: "/etc/letsencrypt/live/supernode.your-domain.com/privkey.pem"
  
registry:
  dht:
    type: "HNSW"
    dimension: 384
    max_elements: 100000
    storage_path: "/var/lib/nexa/data/dht"
    
relay:
  max_connections: 10000
  max_bandwidth: 100Mbps  # per connection
  buffer_size: 64KB
  
stun:
  listen_port: 3478
  public_ip: "your-public-ip"
  
turn:
  listen_port: 5349
  max_sessions: 1000
  session_timeout: 3600
  
replication:
  peers:
    - "supernode-2.your-domain.com:443"
    - "supernode-3.your-domain.com:443"
  sync_interval: 30  # seconds
  
logging:
  level: "info"
  format: "json"
  output: "/var/lib/nexa/logs/supernode.log"
  
monitoring:
  enabled: true
  metrics_port: 9090
  health_check_port: 8080
```

#### 3.2.3 Docker Compose 部署

```yaml
# /opt/nexa/supernode/docker-compose.yaml
version: "3.8"

services:
  supernode:
    image: nexa-net/supernode:latest
    container_name: nexa-supernode
    restart: unless-stopped
    ports:
      - "443:443"
      - "7070:7070"
      - "3478:3478/udp"
      - "5349:5349"
      - "9090:9090"  # Prometheus metrics
      - "8080:8080"  # Health check
    volumes:
      - ./config.yaml:/etc/nexa/config.yaml:ro
      - /etc/letsencrypt:/etc/letsencrypt:ro
      - /var/lib/nexa/data:/var/lib/nexa/data
      - /var/lib/nexa/logs:/var/lib/nexa/logs
    environment:
      - NEXA_LOG_LEVEL=info
      - NEXA_REGION=asia-east
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 10s
      retries: 3
    networks:
      - nexa-network

  # Prometheus for metrics collection
  prometheus:
    image: prom/prometheus:latest
    container_name: nexa-prometheus
    restart: unless-stopped
    ports:
      - "9091:9090"
    volumes:
      - ./prometheus.yaml:/etc/prometheus/prometheus.yml
      - prometheus-data:/prometheus
    networks:
      - nexa-network

  # Grafana for visualization
  grafana:
    image: grafana/grafana:latest
    container_name: nexa-grafana
    restart: unless-stopped
    ports:
      - "3000:3000"
    volumes:
      - grafana-data:/var/lib/grafana
      - ./grafana-dashboards:/etc/grafana/provisioning/dashboards
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=admin
    networks:
      - nexa-network

networks:
  nexa-network:
    driver: bridge

volumes:
  prometheus-data:
  grafana-data:
```

#### 3.2.4 启动服务

```bash
# 启动 Supernode
cd /opt/nexa/supernode
docker compose up -d

# 检查状态
docker compose ps
docker compose logs -f supernode

# 健康检查
curl http://localhost:8080/health
```

### 3.3 高可用配置

#### 3.3.1 多 Supernode 集群

```yaml
# supernode-cluster.yaml
cluster:
  name: "nexa-cluster-1"
  
  nodes:
    - id: "supernode-1"
      role: "primary"
      address: "supernode-1.your-domain.com"
      priority: 100
      
    - id: "supernode-2"
      role: "secondary"
      address: "supernode-2.your-domain.com"
      priority: 50
      
    - id: "supernode-3"
      role: "secondary"
      address: "supernode-3.your-domain.com"
      priority: 50
      
  failover:
    enabled: true
    heartbeat_interval: 10  # seconds
    timeout: 30  # seconds
    auto_failover: true
    
  load_balancing:
    strategy: "round-robin"
    health_check_interval: 30
```

#### 3.3.2 数据同步

```python
class SupernodeReplication:
    """Supernode 数据同步"""
    
    def __init__(self, config: ClusterConfig):
        self.config = config
        self.peers = config.nodes
        self.sync_interval = config.sync_interval
    
    async def sync_dht(self):
        """同步 DHT 数据"""
        for peer in self.peers:
            if peer.role == "secondary":
                # 获取本地变更
                changes = await self.get_local_changes()
                
                # 推送到 peer
                await self.push_changes(peer, changes)
    
    async def sync_registry(self):
        """同步注册数据"""
        for peer in self.peers:
            # 拉取 peer 的注册数据
            peer_data = await self.pull_registry(peer)
            
            # 合并到本地
            await self.merge_registry(peer_data)
```

---

## 4. Nexa-Proxy 部署

### 4.1 系统要求

#### 4.1.1 硬件要求

```yaml
# proxy_hardware.yaml
minimum:
  cpu: 1 core
  memory: 2 GB
  storage: 10 GB
  network: 100 Mbps

recommended:
  cpu: 2 cores
  memory: 4 GB
  storage: 20 GB
  network: 1 Gbps
```

#### 4.1.2 软件要求

```yaml
# proxy_software.yaml
os:
  - Any Linux distribution
  - macOS 12+
  - Windows 10+ (WSL2)

dependencies:
  - Docker (optional)
  - Python 3.11+ (for SDK)
  - Node.js 20+ (for SDK)
```

### 4.2 部署方式

#### 4.2.1 Docker 部署（推荐）

```bash
# 拉取镜像
docker pull nexa-net/nexa-proxy:latest

# 运行容器
docker run -d \
  --name nexa-proxy \
  --restart unless-stopped \
  -p 127.0.0.1:7070:7070 \
  -v /var/lib/nexa:/var/lib/nexa \
  -v ./config.yaml:/etc/nexa/config.yaml \
  nexa-net/nexa-proxy:latest
```

#### 4.2.2 二进制部署

```bash
# 下载二进制
wget https://releases.nexa-net.io/nexa-proxy-linux-x64-v1.0.0.tar.gz
tar -xzf nexa-proxy-linux-x64-v1.0.0.tar.gz

# 安装
sudo mv nexa-proxy /usr/local/bin/
sudo chmod +x /usr/local/bin/nexa-proxy

# 创建配置目录
mkdir -p ~/.nexa

# 生成配置
nexa-proxy init --config ~/.nexa/config.yaml

# 启动服务
nexa-proxy start --config ~/.nexa/config.yaml
```

#### 4.2.3 Systemd 服务

```ini
# /etc/systemd/system/nexa-proxy.service
[Unit]
Description=Nexa-net Proxy Service
After=network.target

[Service]
Type=simple
User=nexa
Group=nexa
ExecStart=/usr/local/bin/nexa-proxy start --config /home/nexa/.nexa/config.yaml
Restart=on-failure
RestartSec=10

[Install]
WantedBy=multi-user.target
```

```bash
# 启用服务
sudo systemctl enable nexa-proxy
sudo systemctl start nexa-proxy
sudo systemctl status nexa-proxy
```

### 4.3 配置文件

```yaml
# ~/.nexa/config.yaml
proxy:
  id: "proxy-$(hostname)"
  version: "1.0.0"
  
identity:
  did: ""  # 自动生成或指定已有 DID
  key_file: "~/.nexa/keys/private.key"
  key_algorithm: "Ed25519"
  
network:
  supernodes:
    - "supernode-1.your-domain.com:443"
    - "supernode-2.your-domain.com:443"
    - "supernode-3.your-domain.com:443"
  listen_address: "127.0.0.1"
  listen_port: 7070
  connection_timeout: 5000  # ms
  heartbeat_interval: 30  # seconds
  
capabilities:
  schema_file: "~/.nexa/capabilities.yaml"
  auto_register: true
  
economy:
  default_budget: 1000
  max_channel_balance: 10000
  token_file: "~/.nexa/tokens.json"
  
logging:
  level: "info"
  format: "json"
  output: "~/.nexa/logs/proxy.log"
  
monitoring:
  enabled: true
  metrics_port: 9092
```

### 4.4 能力配置

```yaml
# ~/.nexa/capabilities.yaml
nexa_capability:
  version: "1.0.0"
  metadata:
    name: "My Agent Services"
    description: "Document processing and translation services"
    tags: ["document", "translation", "nlp"]
    
  endpoints:
    - id: "process_document"
      name: "Document Processing"
      description: "Process and analyze documents"
      input_schema:
        type: object
        properties:
          document:
            type: binary
            max_size: 10MB
          options:
            type: object
            properties:
              extract_text: boolean
              summarize: boolean
      output_schema:
        type: object
        properties:
          text: string
          summary: string
          metadata: object
      cost:
        model: "per_document"
        base_price: 10
```

---

## 5. 容器化部署

### 5.1 Kubernetes 部署

#### 5.1.1 Supernode Deployment

```yaml
# supernode-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: nexa-supernode
  namespace: nexa-net
spec:
  replicas: 3
  selector:
    matchLabels:
      app: nexa-supernode
  template:
    metadata:
      labels:
        app: nexa-supernode
    spec:
      containers:
      - name: supernode
        image: nexa-net/supernode:latest
        ports:
        - containerPort: 443
          name: https
        - containerPort: 7070
          name: nexa
        - containerPort: 3478
          name: stun
          protocol: UDP
        - containerPort: 9090
          name: metrics
        volumeMounts:
        - name: config
          mountPath: /etc/nexa/config.yaml
          subPath: config.yaml
        - name: data
          mountPath: /var/lib/nexa/data
        - name: tls
          mountPath: /etc/letsencrypt
          readOnly: true
        resources:
          requests:
            cpu: "4"
            memory: "16Gi"
          limits:
            cpu: "8"
            memory: "32Gi"
        livenessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /ready
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 5
      volumes:
      - name: config
        configMap:
          name: supernode-config
      - name: data
        persistentVolumeClaim:
          claimName: supernode-data
      - name: tls
        secret:
          secretName: supernode-tls
---
apiVersion: v1
kind: Service
metadata:
  name: nexa-supernode
  namespace: nexa-net
spec:
  type: LoadBalancer
  ports:
  - port: 443
    targetPort: 443
    name: https
  - port: 7070
    targetPort: 7070
    name: nexa
  - port: 3478
    targetPort: 3478
    name: stun
    protocol: UDP
  selector:
    app: nexa-supernode
```

#### 5.1.2 Nexa-Proxy DaemonSet

```yaml
# proxy-daemonset.yaml
apiVersion: apps/v1
kind: DaemonSet
metadata:
  name: nexa-proxy
  namespace: nexa-net
spec:
  selector:
    matchLabels:
      app: nexa-proxy
  template:
    metadata:
      labels:
        app: nexa-proxy
    spec:
      containers:
      - name: proxy
        image: nexa-net/nexa-proxy:latest
        ports:
        - containerPort: 7070
          hostPort: 7070
          name: nexa
        volumeMounts:
        - name: config
          mountPath: /etc/nexa/config.yaml
          subPath: config.yaml
        - name: data
          mountPath: /var/lib/nexa
        resources:
          requests:
            cpu: "500m"
            memory: "2Gi"
          limits:
            cpu: "1"
            memory: "4Gi"
      volumes:
      - name: config
        configMap:
          name: proxy-config
      - name: data
        hostPath:
          path: /var/lib/nexa
          type: DirectoryOrCreate
```

### 5.2 Helm Chart

```yaml
# Chart.yaml
apiVersion: v2
name: nexa-net
version: 1.0.0
description: Nexa-net deployment Helm chart
type: application

# values.yaml
supernode:
  replicaCount: 3
  image:
    repository: nexa-net/supernode
    tag: latest
  resources:
    requests:
      cpu: 4
      memory: 16Gi
  config:
    region: asia-east
    role: primary
    
proxy:
  image:
    repository: nexa-net/nexa-proxy
    tag: latest
  resources:
    requests:
      cpu: 500m
      memory: 2Gi
  config:
    supernodes:
      - supernode-1.example.com
      - supernode-2.example.com
      
monitoring:
  enabled: true
  prometheus:
    enabled: true
  grafana:
    enabled: true
```

---

## 6. 监控与告警

### 6.1 监控指标

#### 6.1.1 Supernode 指标

| 指标 | 描述 | 告警阈值 |
|------|------|----------|
| `supernode_connections` | 当前连接数 | > 8000 |
| `supernode_cpu_usage` | CPU 使用率 | > 80% |
| `supernode_memory_usage` | 内存使用率 | > 80% |
| `supernode_dht_size` | DHT 条目数 | > 90% capacity |
| `supernode_query_latency` | 查询延迟 | > 100ms |
| `supernode_relay_bandwidth` | 中继带宽 | > 80% limit |
| `supernode_error_rate` | 错误率 | > 1% |

#### 6.1.2 Nexa-Proxy 指标

| 指标 | 描述 | 告警阈值 |
|------|------|----------|
| `proxy_rpc_calls` | RPC 调用数 | - |
| `proxy_rpc_latency` | RPC 延迟 | > 500ms |
| `proxy_channel_balance` | 通道余额 | < 100 NEXA |
| `proxy_budget_usage` | 预算使用率 | > 80% |
| `proxy_error_count` | 错误计数 | > 10/min |
| `proxy_connection_status` | 连接状态 | disconnected |

### 6.2 Prometheus 配置

```yaml
# prometheus.yaml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

scrape_configs:
  - job_name: 'supernode'
    static_configs:
      - targets:
        - 'supernode-1:9090'
        - 'supernode-2:9090'
        - 'supernode-3:9090'
        
  - job_name: 'proxy'
    static_configs:
      - targets:
        - 'proxy-1:9092'
        - 'proxy-2:9092'
        
rule_files:
  - 'alerts.yaml'

alerting:
  alertmanagers:
    - static_configs:
        - targets:
          - 'alertmanager:9093'
```

### 6.3 告警规则

```yaml
# alerts.yaml
groups:
  - name: supernode
    rules:
      - alert: SupernodeHighCPU
        expr: supernode_cpu_usage > 80
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Supernode CPU usage high"
          description: "Supernode {{ $labels.instance }} CPU usage is {{ $value }}%"
          
      - alert: SupernodeHighMemory
        expr: supernode_memory_usage > 80
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Supernode memory usage high"
          
      - alert: SupernodeDown
        expr: up{job="supernode"} == 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "Supernode is down"
          
  - name: proxy
    rules:
      - alert: ProxyLowBalance
        expr: proxy_channel_balance < 100
        for: 1m
        labels:
          severity: warning
        annotations:
          summary: "Proxy channel balance low"
          
      - alert: ProxyHighErrorRate
        expr: rate(proxy_error_count[5m]) > 10
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Proxy error rate high"
```

### 6.4 Grafana Dashboard

```json
{
  "dashboard": {
    "title": "Nexa-net Overview",
    "panels": [
      {
        "title": "Supernode Connections",
        "type": "graph",
        "targets": [
          {
            "expr": "supernode_connections",
            "legendFormat": "{{ instance }}"
          }
        ]
      },
      {
        "title": "RPC Latency",
        "type": "heatmap",
        "targets": [
          {
            "expr": "proxy_rpc_latency"
          }
        ]
      },
      {
        "title": "Channel Balances",
        "type": "stat",
        "targets": [
          {
            "expr": "proxy_channel_balance"
          }
        ]
      }
    ]
  }
}
```

---

## 7. 日志管理

### 7.1 日志格式

```json
{
  "timestamp": "2026-03-30T07:00:00.000Z",
  "level": "info",
  "component": "supernode",
  "instance": "supernode-1",
  "message": "New connection established",
  "context": {
    "peer_did": "did:nexa:abc123...",
    "connection_id": "conn-xyz",
    "remote_address": "192.168.1.100:7070"
  },
  "trace_id": "trace-123",
  "span_id": "span-456"
}
```

### 7.2 日志级别

| 级别 | 描述 | 使用场景 |
|------|------|----------|
| `debug` | 详细调试信息 | 开发调试 |
| `info` | 正常操作信息 | 生产环境 |
| `warn` | 警告信息 | 需要关注 |
| `error` | 错误信息 | 需要处理 |
| `fatal` | 致命错误 | 服务终止 |

### 7.3 日志收集

#### 7.3.1 ELK Stack

```yaml
# filebeat.yaml
filebeat.inputs:
  - type: log
    paths:
      - /var/lib/nexa/logs/*.log
    json.keys_under_root: true
    json.add_error_key: true
    
output.elasticsearch:
  hosts: ["elasticsearch:9200"]
  index: "nexa-net-%{+yyyy.MM.dd}"
  
setup.kibana:
  host: "kibana:5601"
```

#### 7.3.2 Loki Stack

```yaml
# promtail.yaml
server:
  http_listen_port: 9080

positions:
  filename: /tmp/positions.yaml

clients:
  - url: http://loki:3100/loki/api/v1/push

scrape_configs:
  - job_name: nexa
    static_configs:
      - targets:
        - localhost
        labels:
          job: nexa-net
          __path__: /var/lib/nexa/logs/*.log
```

---

## 8. 故障排查

### 8.1 常见问题

#### 8.1.1 Supernode 问题

| 问题 | 症状 | 解决方案 |
|------|------|----------|
| **无法启动** | 服务启动失败 | 检查配置文件、端口占用 |
| **连接超时** | Agent 无法连接 | 检查防火墙、TLS 证书 |
| **DHT 查询慢** | 路由延迟高 | 检查索引大小、内存使用 |
| **数据同步失败** | Supernode 数据不一致 | 检查网络连接、同步配置 |

#### 8.1.2 Nexa-Proxy 问题

| 问题 | 症状 | 解决方案 |
|------|------|----------|
| **DID 生成失败** | 无法创建身份 | 检查密钥文件权限 |
| **注册失败** | 能力无法注册 | 检查 Schema 格式 |
| **通道无法开启** | 经济层错误 | 检查余额、保证金 |
| **RPC 调用失败** | 服务调用超时 | 检查网络、目标状态 |

### 8.2 诊断命令

```bash
# Supernode 健康检查
curl http://supernode:8080/health
curl http://supernode:8080/ready

# Supernode 状态
curl http://supernode:8080/status

# DHT 状态
curl http://supernode:8080/dht/status

# Proxy 状态
nexa-proxy status

# Proxy 连接检查
nexa-proxy check-connection --supernode supernode-1

# Proxy 通道状态
nexa-proxy channel list
nexa-proxy channel status <channel-id>

# 日志查看
docker logs nexa-supernode -f
docker logs nexa-proxy -f

# 网络诊断
nexa-proxy diagnose --network
nexa-proxy diagnose --tls
nexa-proxy diagnose --routing
```

### 8.3 故障处理流程

```
┌─────────────────────────────────────────────────────────────┐
│                    Troubleshooting Flow                     │
│                                                             │
│  1. 确认问题                                                 │
│     - 收集症状描述                                           │
│     - 检查监控告警                                           │
│     - 查看日志                                               │
│                                                             │
│  2. 定位原因                                                 │
│     - 检查网络连接                                           │
│     - 检查配置文件                                           │
│     - 检查资源使用                                           │
│     - 检查依赖服务                                           │
│                                                             │
│  3. 解决问题                                                 │
│     - 应用修复方案                                           │
│     - 重启服务                                               │
│     - 更新配置                                               │
│                                                             │
│  4. 验证恢复                                                 │
│     - 检查服务状态                                           │
│     - 测试功能                                               │
│     - 监控指标                                               │
│                                                             │
│  5. 记录总结                                                 │
│     - 记录问题和解决方案                                     │
│     - 更新文档                                               │
│     - 添加预防措施                                           │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## 9. 运维最佳实践

### 9.1 安全运维

1. **定期更新**
   - 每月检查安全更新
   - 及时应用补丁
   - 测试后部署

2. **访问控制**
   - 限制管理接口访问
   - 使用强密码
   - 定期轮换密钥

3. **备份策略**
   - 每日备份 DHT 数据
   - 每周备份配置
   - 定期测试恢复

### 9.2 性能优化

1. **资源调优**
   - 根据负载调整资源
   - 监控瓶颈指标
   - 定期清理数据

2. **网络优化**
   - 使用 CDN 加速
   - 优化 TLS 配置
   - 启用连接复用

3. **缓存策略**
   - 启用 DHT 缓存
   - 配置合理 TTL
   - 监控缓存效率

### 9.3 容灾规划

1. **多区域部署**
   - 至少 2 个区域
   - 数据异步复制
   - 跨区域路由

2. **故障转移**
   - 自动故障检测
   - 快速切换机制
   - 数据一致性保证

3. **恢复演练**
   - 定期演练故障恢复
   - 记录恢复时间
   - 优化恢复流程

---

## 10. 相关文档

### 架构设计

- [整体架构设计](./ARCHITECTURE.md) - 四层架构总览
- [身份与零信任网络层](./IDENTITY_LAYER.md) - Layer 1 设计
- [语义发现与能力路由层](./DISCOVERY_LAYER.md) - Layer 2 设计
- [传输与协商协议层](./TRANSPORT_LAYER.md) - Layer 3 设计
- [资源管理与微交易层](./ECONOMY_LAYER.md) - Layer 4 设计

### 运维相关

- [安全设计规范](./SECURITY.md) - 安全运维指南
- [API 参考](./API_REFERENCE.md) - 运维 API
- [开发者接入指南](./DEVELOPER_GUIDE.md) - Agent 部署指南

### 参考资料

- [Docker Documentation](https://docs.docker.com/)
- [Kubernetes Documentation](https://kubernetes.io/docs/)
- [Prometheus Monitoring](https://prometheus.io/docs/)
- [Grafana Dashboards](https://grafana.com/docs/)