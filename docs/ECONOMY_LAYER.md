# Nexa-net 资源管理与微交易层

> **版本:** v1.0.0-draft | **最后更新:** 2026-03-30
> **所属架构层:** Layer 4 - M2M Economy Layer

## 目录

- [1. 概述](#1-概述)
- [2. Nexa-Token 经济模型](#2-nexa-token-经济模型)
- [3. 状态通道 (State Channels)](#3-状态通道-state-channels)
- [4. 微交易收据 (Micro-Receipt)](#4-微交易收据-micro-receipt)
- [5. 预算控制与资源护栏](#5-预算控制与资源护栏)
- [6. 结算与争议处理](#6-结算与争议处理)
- [7. 实现规范](#7-实现规范)
- [8. 安全考量](#8-安全考量)
- [9. 相关文档](#9-相关文档)

---

## 1. 概述

### 1.1 设计背景

在去中心化 Agent 网络中，没有免费的算力。如果 100 个 Agent 互相调用，必须内置经济护栏：

| 问题 | 无经济机制 | 影响 |
|------|------------|------|
| **资源滥用** | 无限制调用 | 算力被耗尽 |
| **死循环调用** | Agent A→B→A→B... | 网络瘫痪 |
| **服务质量** | 无激励机制 | 服务质量不稳定 |
| **结算延迟** | 每次调用都结算 | 不可接受的延迟 |

### 1.2 设计目标

Nexa-net 经济层设计目标：

1. **高频微交易 (High-Frequency Micro-transactions)** - 支持 10 万+ TPS
2. **低延迟结算 (Low-Latency Settlement)** - 本地即时结算
3. **资源护栏 (Resource Guardrails)** - 防止资源滥用和死循环
4. **激励对齐 (Incentive Alignment)** - 服务提供者获得合理收益

### 1.3 核心概念

```
┌─────────────────────────────────────────────────────────────┐
│              Economy Layer Core Concepts                    │
│                                                             │
│  Nexa-Token (NEXA)                                          │
│  └─────────────────────────────────────────────────────    │
│  Nexa-net 的原生代币，用于支付服务调用费用                    │
│  1 NEXA = 1 单位计算资源（可配置）                           │
│                                                             │
│  State Channel (状态通道)                                   │
│  └─────────────────────────────────────────────────────    │
│  两个 Agent 之间的预锁定信用通道，支持高频本地结算            │
│  无需每次调用都上链                                          │
│                                                             │
│  Micro-Receipt (微交易收据)                                 │
│  └─────────────────────────────────────────────────────    │
│  每次服务调用后生成的签名收据，记录费用和结果                 │
│  双方签名确认                                                │
│                                                             │
│  Budget (预算)                                              │
│  └─────────────────────────────────────────────────────    │
│  Agent 发起调用时设置的最大费用上限                          │
│  超出预算自动终止                                            │
│                                                             │
│  Settlement (结算)                                          │
│  └─────────────────────────────────────────────────────    │
│  状态通道关闭时，将净余额提交到全局账本                       │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### 1.4 层级架构

```
┌─────────────────────────────────────────────────────────────┐
│                   Layer 4: Economy Layer                    │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Token Engine                            │   │
│  │  - Token 发行                                        │   │
│  │  - 余额管理                                          │   │
│  │  - Token 转换                                        │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Channel Manager                         │   │
│  │  - 通道开启                                          │   │
│  │  - 通道维护                                          │   │
│  │  - 通道关闭                                          │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Receipt Engine                          │   │
│  │  - 收据生成                                          │   │
│  │  - 收据签名                                          │   │
│  │  - 收据验证                                          │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Budget Controller                       │   │
│  │  - 预算设置                                          │   │
│  │  - 实时监控                                          │   │
│  │  - 超限处理                                          │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Settlement Engine                       │   │
│  │  - 余额计算                                          │   │
│  │  - 争议仲裁                                          │   │
│  │  - 最终结算                                          │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

---

## 2. Nexa-Token 经济模型

### 2.1 Token 定义

#### 2.1.1 基本属性

```yaml
# token_definition.yaml
nexa_token:
  name: "Nexa Token"
  symbol: "NEXA"
  decimals: 6  # 支持微交易精度
  
  # Token 类型
  type: "utility"  # 功能型代币
  
  # 发行机制
  issuance:
    initial_supply: 1_000_000_000  # 10 亿初始供应
    max_supply: 10_000_000_000     # 100 亿最大供应
    inflation_rate: 0.05           # 5% 年通胀率（用于激励）
    
  # 用途
  purposes:
    - "service_payment"     # 服务支付
    - "channel_collateral"  # 通道保证金
    - "staking_reward"      # 质押奖励
```

#### 2.1.2 Token 价值锚定

Nexa-Token 的价值锚定计算资源：

```
1 NEXA ≈ 1 单位计算资源

单位计算资源定义（可配置）：
- CPU: 1ms 计算时间
- Memory: 1KB 内存使用
- Network: 1KB 数据传输
- Storage: 1KB 存储

实际定价由服务提供者根据成本调整
```

### 2.2 Token 发行机制

#### 2.2.1 初始分配

```
┌─────────────────────────────────────────────────────────────┐
│                    Initial Token Distribution               │
│                                                             │
│  总供应量: 1,000,000,000 NEXA                                │
│                                                             │
│  ┌──────────────────────────────────────────────────────┐  │
│  │                                                      │  │
│  │  ████████████████████████████  40%  Foundation       │  │
│  │  (400M NEXA) - 用于生态建设和运营                      │  │
│  │                                                      │  │
│  │  ████████████████████████████  30%  Community        │  │
│  │  (300M NEXA) - 用于社区激励和空投                      │  │
│  │                                                      │  │
│  │  ████████████████████████████  20%  Service Providers│  │
│  │  (200M NEXA) - 用于早期服务提供者激励                  │  │
│  │                                                      │  │
│  │  ████████████████████████████  10%  Development      │  │
│  │  (100M NEXA) - 用于核心开发团队                       │  │
│  │                                                      │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

#### 2.2.2 通胀机制

```python
class TokenIssuance:
    """Token 发行管理"""
    
    def __init__(self, initial_supply: int, inflation_rate: float):
        self.current_supply = initial_supply
        self.inflation_rate = inflation_rate
        self.last_issuance = datetime.utcnow()
    
    def calculate_new_tokens(self, period_days: int) -> int:
        """计算通胀期间新增 Token"""
        # 年化通胀率转换为期间通胀
        period_rate = self.inflation_rate * (period_days / 365)
        new_tokens = int(self.current_supply * period_rate)
        return new_tokens
    
    def issue_inflation_tokens(self) -> int:
        """发行通胀 Token"""
        now = datetime.utcnow()
        days_since_last = (now - self.last_issuance).days
        
        if days_since_last >= 30:  # 每月发行一次
            new_tokens = self.calculate_new_tokens(days_since_last)
            self.current_supply += new_tokens
            self.last_issuance = now
            return new_tokens
        
        return 0
```

### 2.3 Token 余额管理

#### 2.3.1 余额结构

```typescript
interface TokenBalance {
  // DID 标识
  did: string;
  
  // 总余额
  total: number;
  
  // 可用余额（未锁定）
  available: number;
  
  // 锁定余额（在通道中）
  locked: number;
  
  // 待结算余额
  pending: number;
  
  // 最后更新时间
  lastUpdated: Date;
}

interface BalanceChange {
  // 变化类型
  type: "deposit" | "withdraw" | "lock" | "unlock" | "transfer" | "settlement";
  
  // 变化金额
  amount: number;
  
  // 时间戳
  timestamp: Date;
  
  // 关联交易
  transactionId: string;
  
  // 备注
  note: string;
}
```

#### 2.3.2 余额操作

```python
class BalanceManager:
    """余额管理器"""
    
    def __init__(self, storage: BalanceStorage):
        self.storage = storage
    
    async def get_balance(self, did: str) -> TokenBalance:
        """获取余额"""
        return await self.storage.get(did)
    
    async def deposit(self, did: str, amount: int) -> TokenBalance:
        """充值"""
        balance = await self.get_balance(did)
        balance.total += amount
        balance.available += amount
        await self.storage.update(did, balance)
        return balance
    
    async def lock(self, did: str, amount: int) -> TokenBalance:
        """锁定余额（用于通道）"""
        balance = await self.get_balance(did)
        
        if balance.available < amount:
            raise InsufficientBalanceError(f"Available: {balance.available}, Required: {amount}")
        
        balance.available -= amount
        balance.locked += amount
        await self.storage.update(did, balance)
        return balance
    
    async def unlock(self, did: str, amount: int) -> TokenBalance:
        """解锁余额"""
        balance = await self.get_balance(did)
        balance.locked -= amount
        balance.available += amount
        await self.storage.update(did, balance)
        return balance
```

---

## 3. 状态通道 (State Channels)

### 3.1 通道概述

状态通道是 Nexa-net 经济层的核心机制，实现高频微交易：

```
┌─────────────────────────────────────────────────────────────┐
│                    State Channel Overview                   │
│                                                             │
│  传统方案：每次调用都上链                                    │
│  ┌─────────┐     ┌─────────┐     ┌─────────┐               │
│  │ Agent A │────▶│Blockchain│────▶│ Agent B │               │
│  └─────────┘     └─────────┘     └─────────┘               │
│  延迟: 10-60秒, 成本: 高                                     │
│                                                             │
│  Nexa-net 方案：状态通道                                     │
│  ┌─────────┐     ┌─────────────────────┐     ┌─────────┐   │
│  │ Agent A │────▶│   State Channel     │────▶│ Agent B │   │
│  └─────────┘     │  (本地高频结算)      │     └─────────┘   │
│                  │                     │                    │
│                  │  仅开启/关闭时上链   │                    │
│                  └─────────────────────┘                    │
│  延迟: < 5ms, 成本: 接近零                                   │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### 3.2 通道生命周期

```
┌─────────────────────────────────────────────────────────────┐
│                    Channel Lifecycle                        │
│                                                             │
│  ┌─────────┐                                               │
│  │  IDLE   │  通道未创建                                    │
│  └─────────┘                                               │
│       │                                                     │
│       │ open_channel()                                      │
│       ▼                                                     │
│  ┌─────────┐     ┌─────────────────────────────────────┐   │
│  │ OPENING │────▶│ 1. 双方协商保证金                      │   │
│  └─────────┘     │ 2. 锁定保证金                          │   │
│       │          │ 3. 签署通道合约                        │   │
│       │          │ 4. 提交到账本（可选）                   │   │
│       ▼          └─────────────────────────────────────┘   │
│  ┌─────────┐                                               │
│  │  OPEN   │  通道活跃，可进行微交易                        │
│  └─────────┘                                               │
│       │                                                     │
│       │ 多次 micro_transaction()                           │
│       │                                                     │
│       ▼                                                     │
│  ┌─────────┐                                               │
│  │ ACTIVE  │  通道繁忙                                      │
│  └─────────┘                                               │
│       │                                                     │
│       │ close_channel()                                     │
│       ▼                                                     │
│  ┌─────────┐     ┌─────────────────────────────────────┐   │
│  │ CLOSING │────▶│ 1. 双方签署最终余额                    │   │
│  └─────────┘     │ 2. 提交到账本                          │   │
│       │          │ 3. 解锁保证金                          │   │
│       │          │ 4. 分配净余额                          │   │
│       ▼          └─────────────────────────────────────┘   │
│  ┌─────────┐                                               │
│  │ CLOSED  │  通道已关闭                                    │
│  └─────────┘                                               │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### 3.3 通道开启流程

```
┌─────────────┐                    ┌─────────────┐
│  Nexa-Proxy │                    │  Nexa-Proxy │
│     (A)     │                    │     (B)     │
└──────┬──────┘                    └──────┬──────┘
       │                                  │
       │ 1. CHANNEL_OPEN_REQUEST          │
       │─────────────────────────────────▶│
       │                                  │
       │ {                                │
       │   channel_id: "chan-xyz",        │
       │   deposit_a: 1000,               │
       │   proposed_deposit_b: 1000,      │
       │   timeout: 86400,  # 24 hours    │
       │   settlement_address: "..."      │
       │ }                                │
       │                                  │
       │ 2. CHANNEL_OPEN_RESPONSE         │
       │◀─────────────────────────────────│
       │                                  │
       │ {                                │
       │   accepted: true,                │
       │   deposit_b: 1000,               │
       │   channel_contract: <signed>     │
       │ }                                │
       │                                  │
       │ 3. LOCK_DEPOSIT                  │
       │─────────────────────────────────▶│
       │                                  │
       │ A 锁定 1000 NEXA                  │
       │                                  │
       │ 4. LOCK_DEPOSIT_ACK              │
       │◀─────────────────────────────────│
       │                                  │
       │ B 锁定 1000 NEXA                  │
       │                                  │
       │ 5. CHANNEL_ACTIVE                │
       │─────────────────────────────────▶│
       │                                  │
       │ {                                │
       │   status: "active",              │
       │   total_balance: 2000            │
       │ }                                │
       │                                  │
```

### 3.4 通道合约

#### 3.4.1 合约结构

```protobuf
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
  
  // 通道超时（秒）
  uint64 timeout_seconds = 6;
  
  // 结算地址
  string settlement_address = 7;
  
  // 创建时间
  uint64 created_at = 8;
  
  // 双方签名
  bytes signature_a = 9;
  bytes signature_b = 10;
}

message Party {
  // DID
  string did = 1;
  
  // 公钥
  bytes public_key = 2;
  
  // 结算地址
  string settlement_address = 3;
}
```

#### 3.4.2 合约验证

```python
def validate_channel_contract(contract: ChannelContract) -> bool:
    """验证通道合约"""
    
    # 1. 验证通道 ID 格式
    if not validate_channel_id(contract.channel_id):
        return False
    
    # 2. 验证参与方 DID
    if not validate_did(contract.party_a.did) or not validate_did(contract.party_b.did):
        return False
    
    # 3. 验证保证金
    if contract.deposit_a <= 0 or contract.deposit_b <= 0:
        return False
    
    # 4. 验证超时设置
    if contract.timeout_seconds < 3600:  # 最少 1 小时
        return False
    
    # 5. 验证签名
    if not verify_signature(contract, contract.party_a, contract.signature_a):
        return False
    if not verify_signature(contract, contract.party_b, contract.signature_b):
        return False
    
    return True
```

### 3.5 通道状态管理

```typescript
interface ChannelState {
  // 通道 ID
  channelId: string;
  
  // 状态
  status: "idle" | "opening" | "open" | "active" | "closing" | "closed" | "disputed";
  
  // 参与方
  partyA: Party;
  partyB: Party;
  
  // 保证金
  depositA: number;
  depositB: number;
  
  // 当前余额
  balanceA: number;  // A 在通道中的余额
  balanceB: number;  // B 在通道中的余额
  
  // 累计交易
  totalTransactions: number;
  totalVolume: number;
  
  // 最后更新
  lastUpdate: Date;
  
  // 最后收据序号
  lastReceiptSequence: number;
  
  // 超时时间
  expiresAt: Date;
}
```

---

## 4. 微交易收据 (Micro-Receipt)

### 4.1 收据结构

每次服务调用完成后，双方签署微交易收据：

```protobuf
message MicroReceipt {
  // 收据 ID
  string receipt_id = 1;
  
  // 通道 ID
  string channel_id = 2;
  
  // 序号（递增）
  uint64 sequence = 3;
  
  // 调用方 DID
  string caller_did = 4;
  
  // 服务方 DID
  string provider_did = 5;
  
  // Endpoint ID
  string endpoint_id = 6;
  
  // 调用 ID
  string call_id = 7;
  
  // 费用（NEXA）
  uint64 cost = 8;
  
  // 调用前余额
  BalanceSnapshot balance_before = 9;
  
  // 调用后余额
  BalanceSnapshot balance_after = 10;
  
  // 谹用结果状态
  ReceiptStatus status = 11;
  
  // 时间戳
  uint64 timestamp = 12;
  
  // 调用方签名
  bytes signature_caller = 13;
  
  // 服务方签名
  bytes signature_provider = 14;
}

message BalanceSnapshot {
  uint64 balance_a = 1;
  uint64 balance_b = 2;
}

enum ReceiptStatus {
  SUCCESS = 0;
  PARTIAL_SUCCESS = 1;
  FAILED_NO_CHARGE = 2;
  FAILED_WITH_CHARGE = 3;
  TIMEOUT = 4;
  CANCELLED = 5;
}
```

### 4.2 收据生成流程

```
┌─────────────┐                    ┌─────────────┐
│  Nexa-Proxy │                    │  Nexa-Proxy │
│     (A)     │                    │     (B)     │
│   (Caller)  │                    │  (Provider) │
└──────┬──────┘                    └──────┬──────┘
       │                                  │
       │ RPC Call                         │
       │─────────────────────────────────▶│
       │                                  │
       │                                  │ Execute service
       │                                  │
       │ RPC Response                     │
       │◀─────────────────────────────────│
       │                                  │
       │ {                                │
       │   result: ...,                   │
       │   cost: 25                       │
       │ }                                │
       │                                  │
       │ 1. GENERATE_RECEIPT              │
       │─────────────────────────────────▶│
       │                                  │
       │ {                                │
       │   receipt_id: "rcpt-123",        │
       │   sequence: 42,                  │
       │   cost: 25,                      │
       │   balance_after: {               │
       │     balance_a: 975,              │
       │     balance_b: 1025              │
       │   }                              │
       │ }                                │
       │                                  │
       │ 2. SIGN_RECEIPT (A)              │
       │─────────────────────────────────▶│
       │                                  │
       │ signature_a = sign(receipt)      │
       │                                  │
       │ 3. SIGN_RECEIPT (B)              │
       │◀─────────────────────────────────│
       │                                  │
       │ signature_b = sign(receipt)      │
       │                                  │
       │ 4. RECEIPT_CONFIRMED             │
       │─────────────────────────────────▶│
       │                                  │
       │ 双方保存收据                      │
       │                                  │
```

### 4.3 收据验证

```python
def verify_micro_receipt(
    receipt: MicroReceipt,
    channel_state: ChannelState
) -> tuple[bool, str]:
    """验证微交易收据"""
    
    # 1. 验证通道 ID
    if receipt.channel_id != channel_state.channel_id:
        return False, "Invalid channel ID"
    
    # 2. 验证序号递增
    if receipt.sequence != channel_state.last_receipt_sequence + 1:
        return False, "Invalid sequence number"
    
    # 3. 验证余额变化正确
    expected_balance_a = receipt.balance_before.balance_a - receipt.cost
    expected_balance_b = receipt.balance_before.balance_b + receipt.cost
    
    if receipt.balance_after.balance_a != expected_balance_a:
        return False, "Invalid balance A"
    if receipt.balance_after.balance_b != expected_balance_b:
        return False, "Invalid balance B"
    
    # 4. 验证余额不超限
    if receipt.balance_after.balance_a < 0:
        return False, "Balance A negative"
    if receipt.balance_after.balance_b > channel_state.deposit_a + channel_state.deposit_b:
        return False, "Balance B exceeds total"
    
    # 5. 验证签名
    caller_did = receipt.caller_did
    provider_did = receipt.provider_did
    
    if not verify_receipt_signature(receipt, caller_did, receipt.signature_caller):
        return False, "Invalid caller signature"
    if not verify_receipt_signature(receipt, provider_did, receipt.signature_provider):
        return False, "Invalid provider signature"
    
    return True, "Valid"
```

### 4.4 收据存储

```typescript
interface ReceiptStorage {
  // 存储收据
  store(receipt: MicroReceipt): Promise<void>;
  
  // 获取收据
  get(receiptId: string): Promise<MicroReceipt>;
  
  // 获取通道最新收据
  getLatestReceipt(channelId: string): Promise<MicroReceipt>;
  
  // 获取通道所有收据
  getChannelReceipts(channelId: string): Promise<MicroReceipt[]>;
  
  // 验证收据链完整性
  verifyReceiptChain(channelId: string): Promise<boolean>;
}
```

---

## 5. 预算控制与资源护栏

### 5.1 预算机制

#### 5.1.1 预算设置

```typescript
interface BudgetConfig {
  // 单次调用最大预算
  maxPerCall: number;
  
  // 每分钟最大预算
  maxPerMinute: number;
  
  // 每小时最大预算
  maxPerHour: number;
  
  // 每天最大预算
  maxPerDay: number;
  
  // 总预算上限
  maxTotal: number;
  
  // 预算警告阈值
  warningThreshold: number;  // 如 80%
  
  // 预算耗尽行为
  onExhausted: "reject" | "queue" | "notify";
}
```

#### 5.1.2 预算检查

```python
class BudgetController:
    """预算控制器"""
    
    def __init__(self, config: BudgetConfig):
        self.config = config
        self.usage = {
            "per_minute": 0,
            "per_hour": 0,
            "per_day": 0,
            "total": 0
        }
        self.last_reset = {
            "minute": datetime.utcnow(),
            "hour": datetime.utcnow(),
            "day": datetime.utcnow()
        }
    
    def check_budget(self, requested: int) -> tuple[bool, str]:
        """检查预算是否允许"""
        
        # 重置过期计数器
        self._reset_counters()
        
        # 检查各项限制
        if requested > self.config.maxPerCall:
            return False, "Exceeds per-call limit"
        
        if self.usage["per_minute"] + requested > self.config.maxPerMinute:
            return False, "Exceeds per-minute limit"
        
        if self.usage["per_hour"] + requested > self.config.maxPerHour:
            return False, "Exceeds per-hour limit"
        
        if self.usage["per_day"] + requested > self.config.maxPerDay:
            return False, "Exceeds per-day limit"
        
        if self.usage["total"] + requested > self.config.maxTotal:
            return False, "Exceeds total limit"
        
        return True, "OK"
    
    def record_usage(self, actual: int):
        """记录实际使用"""
        self.usage["per_minute"] += actual
        self.usage["per_hour"] += actual
        self.usage["per_day"] += actual
        self.usage["total"] += actual
        
        # 检查警告阈值
        if self.usage["total"] >= self.config.maxTotal * self.config.warningThreshold:
            self._send_warning()
    
    def _reset_counters(self):
        """重置过期计数器"""
        now = datetime.utcnow()
        
        if (now - self.last_reset["minute"]).total_seconds() >= 60:
            self.usage["per_minute"] = 0
            self.last_reset["minute"] = now
        
        if (now - self.last_reset["hour"]).total_seconds() >= 3600:
            self.usage["per_hour"] = 0
            self.last_reset["hour"] = now
        
        if (now - self.last_reset["day"]).total_seconds() >= 86400:
            self.usage["per_day"] = 0
            self.last_reset["day"] = now
```

### 5.2 资源护栏

#### 5.2.1 死循环检测

```python
class LoopDetector:
    """死循环检测器"""
    
    def __init__(self, max_depth: int = 5):
        self.max_depth = max_depth
        self.call_graph = {}
    
    def detect_loop(self, call_chain: list[str]) -> tuple[bool, list[str]]:
        """检测调用链是否存在循环"""
        
        # 构建调用图
        for i in range(len(call_chain) - 1):
            caller = call_chain[i]
            callee = call_chain[i + 1]
            
            if caller not in self.call_graph:
                self.call_graph[caller] = set()
            self.call_graph[caller].add(callee)
        
        # 检测循环
        visited = set()
        path = []
        
        def dfs(node: str) -> bool:
            if node in path:
                # 找到循环
                loop_start = path.index(node)
                return True, path[loop_start:] + [node]
            
            if node in visited:
                return False, []
            
            visited.add(node)
            path.append(node)
            
            for neighbor in self.call_graph.get(node, set()):
                has_loop, loop_path = dfs(neighbor)
                if has_loop:
                    return True, loop_path
            
            path.pop()
            return False, []
        
        for node in call_chain:
            has_loop, loop_path = dfs(node)
            if has_loop:
                return True, loop_path
        
        return False, []
    
    def check_depth(self, call_chain: list[str]) -> bool:
        """检查调用深度"""
        return len(call_chain) <= self.max_depth
```

#### 5.2.2 调用限制

```typescript
interface CallLimits {
  // 最大调用深度
  maxDepth: number;
  
  // 最大并发调用
  maxConcurrent: number;
  
  // 最大调用频率
  maxRate: number;  // calls per second
  
  // 最大重试次数
  maxRetries: number;
  
  // 调用超时
  timeout: number;  // milliseconds
}

interface CallContext {
  // 调用链
  callChain: string[];
  
  // 当前深度
  depth: number;
  
  // 父调用 ID
  parentCallId: string;
  
  // 预算来源
  budgetSource: string;
  
  // 开始时间
  startTime: Date;
}
```

### 5.3 预算传递

当 Agent A 调用 Agent B，B 又调用 Agent C 时，预算需要传递：

```
┌─────────────────────────────────────────────────────────────┐
│                    Budget Propagation                       │
│                                                             │
│  Agent A (预算: 100 NEXA)                                   │
│       │                                                     │
│       │ 调用 B，预算: 50 NEXA                                │
│       ▼                                                     │
│  Agent B                                                    │
│       │                                                     │
│       │ 调用 C，预算: 20 NEXA (从 A 的预算中分配)            │
│       ▼                                                     │
│  Agent C                                                    │
│       │                                                     │
│       │ 实际花费: 15 NEXA                                    │
│       ▼                                                     │
│  返回结果给 B                                                │
│       │                                                     │
│       │ B 花费: 10 NEXA (自己的服务费)                       │
│       │ C 花费: 15 NEXA (传递给 A)                           │
│       ▼                                                     │
│  返回结果给 A                                                │
│       │                                                     │
│       │ A 总花费: 25 NEXA                                    │
│       ▼                                                     │
│  A 剩余预算: 75 NEXA                                         │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

```python
def propagate_budget(
    parent_budget: int,
    child_request: int,
    service_fee: int
) -> tuple[int, int]:
    """计算预算传递"""
    
    # 子调用预算不能超过父预算
    child_budget = min(child_request, parent_budget - service_fee)
    
    # 确保子预算为正
    if child_budget < 0:
        child_budget = 0
    
    return child_budget, service_fee
```

---

## 6. 结算与争议处理

### 6.1 通道关闭结算

#### 6.1.1 正常关闭

```
┌─────────────┐                    ┌─────────────┐
│  Nexa-Proxy │                    │  Nexa-Proxy │
│     (A)     │                    │     (B)     │
└──────┬──────┘                    └──────┬──────┘
       │                                  │
       │ 1. CHANNEL_CLOSE_REQUEST         │
       │─────────────────────────────────▶│
       │                                  │
       │ {                                │
       │   channel_id: "chan-xyz",        │
       │   final_balance_a: 750,          │
       │   final_balance_b: 1250,         │
       │   last_receipt_sequence: 100     │
       │ }                                │
       │                                  │
       │ 2. CHANNEL_CLOSE_RESPONSE        │
       │◀─────────────────────────────────│
       │                                  │
       │ {                                │
       │   accepted: true,                │
       │   final_balance_a: 750,          │
       │   final_balance_b: 1250          │
       │ }                                │
       │                                  │
       │ 3. FINAL_SETTLEMENT              │
       │─────────────────────────────────▶│
       │                                  │
       │ 双方签署最终结算                  │
       │                                  │
       │ 4. SUBMIT_TO_LEDGER              │
       │─────────────────────────────────▶│
       │                                  │
       │ 提交到全局账本                    │
       │                                  │
       │ 5. UNLOCK_DEPOSITS               │
       │◀─────────────────────────────────│
       │                                  │
       │ A 获得 750 NEXA                   │
       │ B 获得 1250 NEXA                  │
       │                                  │
```

#### 6.1.2 结算计算

```python
def calculate_settlement(
    channel_state: ChannelState,
    final_receipt: MicroReceipt
) -> SettlementResult:
    """计算最终结算"""
    
    # 验证最终收据
    if not verify_micro_receipt(final_receipt, channel_state):
        raise SettlementError("Invalid final receipt")
    
    # 计算净余额
    net_balance_a = final_receipt.balance_after.balance_a
    net_balance_b = final_receipt.balance_after.balance_b
    
    # 计算解锁金额
    unlock_a = net_balance_a
    unlock_b = net_balance_b
    
    return SettlementResult(
        channel_id=channel_state.channel_id,
        party_a=channel_state.party_a.did,
        party_b=channel_state.party_b.did,
        amount_a=unlock_a,
        amount_b=unlock_b,
        total_transactions=channel_state.total_transactions,
        total_volume=channel_state.total_volume
    )
```

### 6.2 争议处理

#### 6.2.1 争议场景

| 场景 | 描述 | 处理方式 |
|------|------|----------|
| **余额不一致** | 双方对最终余额有分歧 | 提交最新收据链验证 |
| **收据缺失** | 一方声称收据丢失 | 使用对方保存的收据 |
| **恶意关闭** | 一方尝试用旧余额关闭 | 使用最新收据反驳 |
| **超时关闭** | 一方无响应 | 单方提交关闭请求 |

#### 6.2.2 争议仲裁

```python
class DisputeResolver:
    """争议仲裁器"""
    
    def resolve_balance_dispute(
        self,
        channel_id: str,
        claim_a: BalanceClaim,
        claim_b: BalanceClaim
    ) -> ArbitrationResult:
        """解决余额争议"""
        
        # 1. 获取双方收据链
        receipts_a = self.storage.get_channel_receipts(channel_id, claim_a.did)
        receipts_b = self.storage.get_channel_receipts(channel_id, claim_b.did)
        
        # 2. 验证收据链完整性
        valid_a = self.verify_receipt_chain(receipts_a)
        valid_b = self.verify_receipt_chain(receipts_b)
        
        # 3. 选择最新有效收据
        if valid_a and valid_b:
            # 双方都有效，选择序号最大的
            latest = max(receipts_a[-1], receipts_b[-1], key=lambda r: r.sequence)
        elif valid_a:
            latest = receipts_a[-1]
        elif valid_b:
            latest = receipts_b[-1]
        else:
            # 双方都无效，使用通道初始状态
            return self.use_initial_state(channel_id)
        
        # 4. 基于最新收据结算
        return ArbitrationResult(
            valid=True,
            final_balance_a=latest.balance_after.balance_a,
            final_balance_b=latest.balance_after.balance_b,
            evidence=latest
        )
    
    def handle_timeout_close(
        self,
        channel_id: str,
        requester: str
    ) -> ArbitrationResult:
        """处理超时关闭"""
        
        # 获取请求方最新收据
        receipts = self.storage.get_channel_receipts(channel_id, requester)
        
        if receipts:
            latest = receipts[-1]
            return ArbitrationResult(
                valid=True,
                final_balance_a=latest.balance_after.balance_a,
                final_balance_b=latest.balance_after.balance_b,
                evidence=latest
            )
        else:
            # 无收据，使用初始状态
            return self.use_initial_state(channel_id)
```

### 6.3 全局账本

#### 6.3.1 账本结构

```typescript
interface GlobalLedger {
  // 账本 ID
  ledgerId: string;
  
  // 账本类型
  type: "blockchain" | "centralized" | "hybrid";
  
  // 账本状态
  status: "active" | "paused" | "maintenance";
  
  // 最后区块/交易号
  lastBlock: number;
}

interface LedgerEntry {
  // 条目 ID
  entryId: string;
  
  // 类型
  type: "channel_open" | "channel_close" | "transfer" | "mint" | "burn";
  
  // 相关 DID
  dids: string[];
  
  // 金额变化
  amounts: Map<string, number>;
  
  // 通道 ID（如适用）
  channelId?: string;
  
  // 时间戳
  timestamp: Date;
  
  // 签名
  signatures: bytes[];
}
```

#### 6.3.2 账本提交

```python
async def submit_settlement_to_ledger(
    settlement: SettlementResult,
    signatures: list[bytes]
) -> LedgerEntry:
    """提交结算到全局账本"""
    
    # 构建账本条目
    entry = LedgerEntry(
        entry_id=generate_entry_id(),
        type="channel_close",
        dids=[settlement.party_a, settlement.party_b],
        amounts={
            settlement.party_a: settlement.amount_a,
            settlement.party_b: settlement.amount_b
        },
        channel_id=settlement.channel_id,
        timestamp=datetime.utcnow(),
        signatures=signatures
    )
    
    # 提交到账本
    ledger_entry = await ledger.submit(entry)
    
    return ledger_entry
```

---

## 7. 实现规范

### 7.1 接口定义

```typescript
interface EconomyLayerAPI {
  // Token 管理
  token: {
    getBalance(did: string): Promise<TokenBalance>;
    deposit(did: string, amount: number): Promise<void>;
    withdraw(did: string, amount: number): Promise<void>;
    transfer(from: string, to: string, amount: number): Promise<void>;
  };
  
  // 通道管理
  channel: {
    open(peerDID: string, deposit: number): Promise<ChannelState>;
    close(channelId: string): Promise<SettlementResult>;
    getState(channelId: string): Promise<ChannelState>;
    listChannels(did: string): Promise<ChannelState[]>;
  };
  
  // 收据管理
  receipt: {
    generate(channelId: string, cost: number): Promise<MicroReceipt>;
    sign(receipt: MicroReceipt): Promise<MicroReceipt>;
    verify(receipt: MicroReceipt): Promise<boolean>;
    getLatest(channelId: string): Promise<MicroReceipt>;
  };
  
  // 预算控制
  budget: {
    setConfig(did: string, config: BudgetConfig): Promise<void>;
    checkBudget(did: string, amount: number): Promise<boolean>;
    recordUsage(did: string, amount: number): Promise<void>;
    getUsage(did: string): Promise<BudgetUsage>;
  };
  
  // 结算
  settlement: {
    calculate(channelId: string): Promise<SettlementResult>;
    submit(settlement: SettlementResult): Promise<LedgerEntry>;
    dispute(channelId: string, claim: BalanceClaim): Promise<ArbitrationResult>;
  };
}
```

### 7.2 错误码

| 错误码 | 描述 | 处理建议 |
|--------|------|----------|
| `EC001` | 余额不足 | 充值或减少调用 |
| `EC002` | 通道未开启 | 先开启通道 |
| `EC003` | 通道已关闭 | 开启新通道 |
| `EC004` | 保证金不足 | 增加保证金 |
| `EC005` | 收据签名无效 | 检查签名 |
| `EC006` | 收据序号错误 | 同步收据链 |
| `EC007` | 预算超限 | 调整预算配置 |
| `EC008` | 调用深度超限 | 简化调用链 |
| `EC009` | 结算争议 | 提交仲裁 |
| `EC010` | 账本提交失败 | 重试或联系管理员 |

---

## 8. 安全考量

### 8.1 威胁模型

| 威胁 | 风险等级 | 缓解措施 |
|------|----------|----------|
| **余额伪造** | 高 | 收据双签名验证 |
| **恶意关闭** | 高 | 最新收据反驳机制 |
| **收据篡改** | 高 | 签名验证 + 序号递增 |
| **通道超时攻击** | 中 | 超时关闭机制 |
| **预算绕过** | 中 | 多层预算检查 |
| **死循环攻击** | 中 | 深度限制 + 循环检测 |

### 8.2 安全最佳实践

1. **收据管理**
   - 每次交易后立即生成并签署收据
   - 本地持久化存储所有收据
   - 定期验证收据链完整性

2. **通道管理**
   - 设置合理的超时时间
   - 监控通道余额变化
   - 及时关闭不活跃通道

3. **预算控制**
   - 设置多层预算限制
   - 实时监控使用情况
   - 设置警告和自动熔断

---

## 9. 相关文档

### 上层架构

- [整体架构设计](./ARCHITECTURE.md) - 四层架构总览
- [传输与协商协议层](./TRANSPORT_LAYER.md) - Layer 3 设计

### 相关规范

- [协议规范](./PROTOCOL_SPEC.md) - 经济层协议定义
- [API 参考](./API_REFERENCE.md) - 经济层 API 定义
- [安全设计](./SECURITY.md) - 经济层安全机制

### 参考资料

- [Lightning Network Protocol](https://lightning.network/)
- [State Channels Overview](https://statechannels.org/)
- [Payment Channels Design](https://eprint.iacr.org/2017/283)