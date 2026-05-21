# Avis / Tauri Threshold Signing Workspace

这是一个用于阈值签名、DKG、VSS、Schnorr 份额证明和 BLS 聚合演示的 Tauri + Vue + Rust 工作区。

前端是一个 Tauri 桌面应用，用于调试和驱动整个协议流程；密码学核心逻辑集中在 `crates/math_core`；示例后端放在 `api/server_example`，用于提供 HTTP 接口和协议验证示例。

## 项目目标

1. 通过 DKG 生成阈值密钥材料，确保没有单点持有完整私钥。
2. 通过 Feldman VSS 验证份额，确保每个子份额都可被公开承诺校验。
3. 通过 Schnorr NIZK 证明份额控制权，确保发起签名请求的一方确实持有合法份额。
4. 通过 BLS 部分签名和聚合，得到可验证的最终签名。
5. 给前端、后端和未来的 AI agent 提供清晰的协议入口和调试面板。

## 当前架构

- `src/`：Tauri 前端，当前是一个面向协议调试的控制台页面。
- `src-tauri/`：Tauri Rust 命令层，负责生成本地 Schnorr 证明等辅助能力。
- `crates/math_core/`：密码学核心库，包含 DKG、VSS、Schnorr 和 BLS 基础实现。
- `api/openapi.yaml`：接口草案，描述健康检查、DKG、证明验证、部分签名和聚合接口。
- `api/server_example/`：axum 示例后端，用于验证前端请求和协议消息格式。

## Flow Chart

```mermaid
sequenceDiagram
    autonumber
    participant User as 用户
    participant AIAudit as AI审计网关(每个MPC节点分别拥有一个)
    participant NodeA as MPC节点 A
    participant NodeB as MPC节点 B
    participant NodeC as MPC节点 C
    participant NodeD as MPC节点 D
    participant Verifier as 验证方

    rect rgb(240, 248, 255)
        note over User,NodeD: 阶段一：DKG 生成份额（无人持有完整私钥）
        User->>User: 1. 生成本地随机种子并参与 DKG, 生成多项式 f_U 并广播承诺 C_U, 得到自己的私钥 SK_U = f_U(0) 和公钥 PK_U
        NodeA->>NodeA: 2. 生成多项式 f_A 并广播承诺 C_A
        NodeB->>NodeB: 3. 生成多项式 f_B 并广播承诺 C_B
        NodeC->>NodeC: 4. 生成多项式 f_C 并广播承诺 C_C
        NodeD->>NodeD: 5. 生成多项式 f_D 并广播承诺 C_D
        Note over User,NodeD: 6. 各方私下交换子份额并验证，得到自己的最终份额 Share_U / Share_A..Share_D
        Note over User, NodeD: 7. 计算全局公钥并公开 PK = g^{F(0)} = C_U[0] * C_A[0] * C_B[0] * C_C[0] * C_D[0]
    end

    rect rgb(255, 250, 240)
        note over User,NodeD: 阶段二：用户发起签名请求，先进入 AI 审计层再进入 MPC 节点
        User->>User: 8. 生成 Schnorr NIZK 证明 自己持有 PK_U 对应的私钥 SK_U
        User->>AIAudit: 9. 发送 Sign_Request(M, Proof_share, nonce, ts)
        AIAudit->>AIAudit: 10. 识别签名请求意图、风险特征与异常上下文（异步）
        AIAudit->>NodeA: 11. 放行给 MPC 节点 A
        AIAudit->>NodeB: 12. 放行给 MPC 节点 B
        AIAudit->>NodeC: 13. 放行给 MPC 节点 C
    end

    rect rgb(245, 255, 250)
        note over AIAudit,NodeD: 阶段三：MPC 节点执行内部审计，并结合 AI 审计层结果生成部分签名
        NodeA->>NodeA: 14. 验证 Proof_share和内部审计规则
        NodeB->>NodeB: 15. 验证 Proof_share和内部审计规则
        NodeC->>NodeC: 16. 验证 Proof_share和内部审计规则
        NodeA-->>AIAudit: 17. 返回 sigma_A 与审计记录
        NodeB-->>AIAudit: 18. 返回 sigma_B 与审计记录
        NodeC-->>AIAudit: 19. 返回 sigma_C 与审计记录
        AIAudit-->>User: 20. 依据审计结果判断是否放行请求
    end

    rect rgb(230, 230, 255)
        note over User,Verifier: 阶段四：聚合与验证
        User->>User: 21. 聚合部分签名得到对应公钥PK的签名 sigma_final
        User->>Verifier: 22. 提交 M 和 sigma_final
        Verifier->>Verifier: 23. 使用公钥 PK 验证
        Verifier-->>User: 24. 验证通过
    end
```

## 当前进展

下面是目前已经落地的内容，便于新加入的人快速判断项目状态：

- DKG/VSS 逻辑已经完成，承诺项使用 `G1Affine`，并且支持份额验证。
- Schnorr 份额控制证明已经实现，可用于证明请求方确实持有自己的 share。
- BLS 签名与聚合路径已经具备基础实现，并接入到示例后端接口。
- 纯 Rust 的测试和模拟器已经建立，用来验证协议链路的可行性。
- OpenAPI 草案已经写好，后端和前端都在向这个契约靠拢。
- Tauri 前端已经从默认模板改成协议工作台，可以直接发请求、看响应、生成证明并聚合签名。
- 示例后端已经能通过编译检查，前端构建也已经通过。

### 最近一次验证结果

- `pnpm build` 通过。
- `cargo check --manifest-path /Users/klizz/Desktop/Tauri/src-tauri/Cargo.toml` 通过。
- `cargo check -p server_example` 通过。

## 演示摘要与 Mock 实现说明

简要说明当前可复现的 mock 演示以及实现要点：

- 当前状态：前端 demo 已跑通（健康检查 → 节点发现 → DKG 引导 → 本地 Schnorr 证明生成 → 后端验证 → 后端返回部分签名 → BLS 聚合并最终验证），UI 上所有步骤显示为 DONE。

- 主要交互流（等同于 `Run full demo` 在 `src/App.vue` 中的顺序）：
    1. `POST /api/v1/dkg/init`：创建并存储一个可复现的 demo round（`MockRoundState`）。
    2. `POST /api/v1/demo/proof`：前端/本地（或后端构造）生成 Schnorr 证明 bundle（包含 `R/s/pk_share` 的 base64、`nonce`、`ts`、message 等），也可通过 `src-tauri` 的 `generate_demo_proof` 命令生成。
    3. `POST /api/v1/proof/verify`：后端解码 bundle，重建挑战并调用 `math_core::schnorr::schnorr_verify` 做验证。验证通过后允许后续部分签名请求。
    4. `POST /api/v1/sign/partial`：后端在验证 proof 后对请求者的份额执行 BLS 部分签名并返回 `sigma`（base64）。
    5. `POST /api/v1/bls/aggregate`：聚合多个部分签名并用阈值公钥验证最终签名。

- 序列化与哈希细节（保证两端一致的关键）：
    - 所有点/标量均使用 `serialize_uncompressed`（ark-serialize）并用标准 Base64 传输。
    - Schnorr 挑战 `c` 的计算：将 `pk_share` (affine uncompressed bytes) || `R` (affine uncompressed bytes) || `message` || `nonce` (u64 little-endian) || `ts` (u64 little-endian) 送入 `Blake2s256`，取输出前 8 字节按 little-endian 解释为 u64，再转为曲线标量作为 `c`。

- Mock 实现要点：
    - `crates/api/server_example/src/main.rs` 中维护一个内存 `MockCluster`，通过 `MockRoundState::build` 使用 `math_core::dkg::simulate_dkg` 生成承诺、份额与公钥，并对 demo user 的份额做可控微调（便于演示）。
    - 前端 `src/App.vue` 按步骤驱动 API 请求并在界面展示每步状态与返回数据。
    - 本地 Tauri 命令 `src-tauri/src/lib.rs::generate_demo_proof` 可用于在桌面端直接生成 proof bundle（等价于后端的 demo bundle 构造）。

- 本地复现命令（等同于前端操作，便于调试）：

```bash
# 1) 获取 demo proof（等同于 UI 的 /demo/proof）
curl -s -X POST -H "Content-Type: application/json" \
    -d '{"round_id":"round-001","message":"threshold signature demo","seed":7,"nonce":12345,"timestamp":1620000000}' \
    http://127.0.0.1:8443/api/v1/demo/proof | jq '.'

# 2) 将该 bundle 原样提交到 verify（等同于 UI 的 /proof/verify）
curl -s -X POST -H "Content-Type: application/json" \
    -d '{"message":"<message_b64>","proof":{"R":"<r_b64>","s":"<s_b64>","pk_share":"<pk_b64>","nonce":12345,"ts":1620000000}}' \
    http://127.0.0.1:8443/api/v1/proof/verify | jq '.'
```

- 调试说明与下一步建议：
    - 为定位偶发的验证失败，已在示例后端的 `verify_proof` 与 `sign_partial` 路径加入临时 stderr debug 输出（打印收到的 `pk_share/R/s` 的 base64、`nonce`/`ts` 与 message 长度），便于对比前端与后端接收到的原始数据。
    - 推荐完成：对比两端计算挑战 `c` 的原始缓冲区字节（`pk||R||message||nonce||ts`）是否精确相等；若不等，修复序列化顺序或字节宽度并移除临时日志。


## 协议简述

### DKG 和 VSS

每个参与方本地选择多项式：

$$
f_i(x) = a_{i,0} + a_{i,1}x + \cdots + a_{i,t-1}x^{t-1}
$$

公开承诺：

$$
C_{i,k} = g^{a_{i,k}}
$$

分发给第 $j$ 方的份额：

$$
s_{i\to j} = f_i(j)
$$

校验式：

$$
g^{s_{i\to j}} \stackrel{?}{=} \prod_{k=0}^{t-1} C_{i,k}^{j^k}
$$

最终每个参与者得到自己的总份额：

$$
S_j = \sum_i s_{i\to j}
$$

全局公钥与私钥常数项一致：

$$
PK = g^{F(0)} = \prod_i C_{i,0}
$$

### 份额控制证明

用户在发起签名前，需要对自己的份额做 Schnorr NIZK 证明：

$$
c = H(PK_{share} \parallel R \parallel M \parallel nonce \parallel ts)
$$

$$
s = k + c \cdot Share_U
$$

节点侧验证：

$$
sG \stackrel{?}{=} R + c \cdot PK_{share}
$$

### 阈值签名聚合

单个部分签名：

$$
\sigma_j = S_j \cdot H(m)
$$

使用拉格朗日系数聚合：

$$
\sigma_{final} = \sum_{j \in \mathcal{T}} \lambda_j \cdot \sigma_j
$$

最终结果等价于完整私钥签名：

$$
\sigma_{final} = SK \cdot H(m)
$$

## 如何运行

### 前端开发

```bash
pnpm dev
```

### Tauri 桌面开发

```bash
pnpm tauri dev
```

### iOS 开发

```bash
pnpm tauri ios dev
```

### 后端示例

如果你要调试 HTTP 接口，可以运行 `api/server_example` 对应的示例服务。

## 当前待办

- [x] DKG/VSS 基础逻辑和验证路径
- [x] Schnorr 份额控制证明
- [x] BLS 部分签名与聚合路径
- [x] Tauri 前端改造为协议工作台
- [x] 示例后端和前端构建通过
- [ ] 把 Tauri 前端和示例后端进一步对齐成稳定的交互流程
- [x] 把部分签名的响应解析、去重和聚合流程做成更完整的用户操作
- [x] 补充端到端模拟：多节点 DKG -> 份额验证 -> 证明 -> 部分签名 -> 聚合
- [ ] 将 API 草案和后端实现进一步收敛，减少示例字段和最终字段之间的偏差
- [ ] 增加更正式的错误处理、审计日志和重放保护展示

## 给后续开发者 / AI agent 的提示

- 优先看 `crates/math_core`，协议正确性都从这里开始。
- 前端目前是一个协议控制台，不是业务产品界面，先保证流程可跑通，再谈收敛和美化。
- `api/openapi.yaml` 是接口契约，改后端之前最好先看这里。
- `tmp/scheme.md` 里已经写了 DKG 和阈值签名的数学说明，适合用来对照实现。
- 如果要继续扩展功能，优先补一个完整的端到端小闭环，而不是零散补页面。