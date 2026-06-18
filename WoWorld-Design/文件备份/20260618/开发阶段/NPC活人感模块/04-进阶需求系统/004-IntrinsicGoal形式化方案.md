> **开发代号**: WoWorld (Wonder World)
> **文档类型**: 参考文档 / 设计探讨草稿
> **主题**: IntrinsicGoal 形式化 —— commitment × relevance → action_weight
> **日期**: 2026-06-15
> **关联**: [[参考文档/第二部分-模块系统基础扩张存档-20260618/025-NPC进阶需求系统设计探讨-20260615/001-理论框架与维度论证]] | [[../Happy Game/开发阶段/NPC活人感模块/NPC活人感开发文档ver2.0|NPC ver2.0 §2.8 SelfNarrative]]

---

# 004 — IntrinsicGoal 形式化方案

## 一、现状与问题

### 1.1 已有但未详述的部分

NPC ver2.0 在概率决策器的权重链中已经定义了：

```
TotalWeight = ... × intrinsic_motivation_weight × ...
```

`IntrinsicGoal` 枚举已定义 6 种，`generate_intrinsic_goals()` 已详述生成逻辑。但：

- ❌ `intrinsic_motivation_weight` 如何从 `IntrinsicGoal` 列表计算——未指定
- ❌ 不同 goal 对不同 action 的"相关性"——未指定
- ❌ goal 的"投入度"（commitment）如何量化——未指定
- ❌ 多个 goal 之间的组合方式——未指定

### 1.2 不对 IntrinsicGoal 做 deficit 化

重申原则（来自 [[参考文档/第二部分-模块系统基础扩张存档-20260618/025-NPC进阶需求系统设计探讨-20260615/001-理论框架与维度论证]]）：
- IntrinsicGoal 是**志向型驱力**——不是匮乏信号
- 保持数学独立性：deficit（标量 urgency）vs aspiration（向量 preference bias）
- 不定义"自主 deficit"或"超越 deficit"——这些概念在数学上不适合 deficit 模型

---

## 二、核心公式

### 2.1 主公式

```
intrinsic_motivation_weight(goals, action) =
    ∏_{g ∈ goals} (1.0 + relevance(g, action) × commitment(g) × 0.5)

最终 clamp 到 [1.0, 3.0]
```

- 如果没有 active goals → 返回 1.0（不产生影响）
- 如果 1 个 goal 完全相关且完全投入 → 1.0 + 1.0×1.0×0.5 = 1.5
- 如果 3 个 goal 都对同一 action 高相关 → 1.5³ = 3.375 → clamp 到 3.0
- 3.0 的上限意味着"强烈被内在目标驱动"的行为相对于"完全无目标驱动"的行为有 3 倍权重

### 2.2 为什么是乘法而非加法？

```
加法: 1.0 + Σ(relevance × commitment × 0.5)
  问题: 多个 weak goals 可以凑出高权重（1+0.1+0.1+0.1+0.1+0.1=1.5）
  这不合理——10个弱关联不应该等价于1个强关联

乘法: ∏(1.0 + relevance × commitment × 0.5)
  效果: 只有真正高度相关且高度投入的 goal 才能显著提升权重
  1.05 × 1.05 × 1.05 × 1.05 × 1.05 = 1.28 (多个弱关联，影响有限)
  1.5 × 1.0 × 1.0 × 1.0 × 1.0 = 1.5 (一个强关联，显著影响)
```

乘法确保了**内在目标的"专注度"有实际意义**——专心致志的 NPC 比分心的 NPC 在特定行为上更被驱动。

---

## 三、commitment(goal) — 投入度函数

### 3.1 通用公式

```
commitment(goal, self_narrative) = base_commitment × age_factor

base_commitment ∈ [0.0, 1.0] — 目标特定的基础投入度
age_factor ∈ [1.0, 1.3] — 年轻 NPC 投入更强烈
```

### 3.2 各 goal 的 base_commitment

| Goal | 公式 | 范围 | 设计理由 |
|------|------|------|---------|
| **ExploreUnknown** | `openness × 0.8` | 0~0.8 | 好奇心几乎完全由开放性决定。上限 0.8——很少有 NPC 对探索的投入度接近 1.0 |
| **MasterSkill** | `progress × 0.4 + (1-progress) × 0.6` | 0.4~1.0 | 已投入越多→越坚持（沉没成本效应）。即使刚开始（progress=0），commitment 也有 0.6——因为生成这个 goal 本身就意味着一定的投入 |
| **ActOnValue** | `max_aligned_value_strength` | 0~1.0 | 由与 goal 对齐的最强 CoreValue 的强度决定 |
| **CreateSomething** | `openness × 0.6 + (1-progress) × 0.4` | 0~1.0 | 类似 MasterSkill 但更依赖开放性（创作需要灵感） |
| **DeepenConnection** | `\|affection\| × 0.3 + (1-trust) × 0.5 + familiarity × 0.2` | 0~1.0 | 情感越深 + 信任越缺（修复关系）+ 越熟悉 → 越投入。对新朋友：0.3+0.5+0.1=0.9（高投入修复或建立）。对老朋友：0.8+0.1+0.9=1.0→clamp 到 1.0 |
| **LeaveLegacy** | `closeness_to_death × 0.7 + stagnation_sense × 0.3` | 0~1.0 | 离死越近+人生越空虚 → 传承欲越强。35 岁健康 NPC：≈0。80 岁 NPC：≈0.7-1.0 |

### 3.3 age_factor

```
age_factor(age_proportion) = 1.0 + (1.0 - age_proportion) × 0.3
// age_proportion = current_age / species_max_age
// 20岁（age_proportion=0.25）→ age_factor=1.225
// 50岁（age_proportion=0.63）→ age_factor=1.11
// 80岁（age_proportion=1.0）→ age_factor=1.0
```

年轻人对目标有更强的情绪投入——这是心理学研究中反复验证的发现。

---

## 四、relevance(goal, action) — 相关性函数

### 4.1 映射表

这是**纯函数**（goal type × action type → f32）——不依赖 NPC 状态，可编译时优化。

| Goal | 强相关 (≥0.7) | 中相关 (0.3~0.6) | 弱相关 (0.1~0.2) |
|------|-------------|-----------------|-----------------|
| **ExploreUnknown** | Explore(0.9), Travel(0.8), Investigate(0.8) | Read(0.5), AskQuestions(0.4), MapMaking(0.5) | GatherRumors(0.2) |
| **MasterSkill(s)** | TrainSkill(s)(1.0) | SeekMentor(s)(0.8), Practice(0.6), TrainSkill(other)(0.3) | Observe(0.2) |
| **ActOnValue** | (动态，见 4.2) | (动态) | — |
| **CreateSomething** | CraftItem(0.9), Build(0.8), Compose(0.9) | GatherMaterials(0.4), Design(0.6) | VisitMarket(0.2) |
| **DeepenConnection(t)** | Visit(t)(1.0), Gift(t)(0.8), Help(t)(0.9) | Socialize(0.5), WriteLetter(t)(0.7) | AttendFestival(0.2) |
| **LeaveLegacy** | Teach(0.9), Write(0.8), MentorChild(1.0) | Build(0.5), CreateHeirloom(0.7), RecruitApprentice(0.8) | Socialize(0.1) |

### 4.2 ActOnValue 的特殊处理

ActOnValue 的 relevance 是**动态的**——取决于 action 是否与 NPC 的 CoreValues 对齐：

```rust
fn act_on_value_relevance(action: &ActionType, core_values: &[CoreValue]) -> f32 {
    core_values.iter()
        .filter(|v| v.aligns_with(action))
        .map(|v| v.strength)
        .max()
        .unwrap_or(0.1)
}

// 例如: CoreValue("honesty", strength=0.8)
//   → 对 Lie/Deceive action 的 relevance = 0.0 (反向对齐)
//   → 对 Confess/Testify action 的 relevance = 0.8 (正向对齐)
//   → 对 Trade action 的 relevance = 0.1 (中性)
```

但这个查询需要 action 能表达其"价值负荷"。v1.0 用 `0.5` 作为 ActOnValue 的默认 relevance（保守估计），v2.0 可实现完整的 value-action 对齐查询。

---

## 五、goal 生命周期对权重的影响

### 5.1 Goal 状态

```rust
enum GoalState {
    Active,       // 正常驱动行为
    Dormant,      // 暂停（外部条件不满足，如冬季无法旅行）
    Abandoned,    // 放弃（frustration 过高导致）
    Achieved,     // 完成（progress = 1.0）
}
```

- `Active` → 正常贡献权重
- `Dormant` → commitment × 0.3（仍然想要，但被现实阻碍）
- `Abandoned` → 不贡献权重（已从 active goals 列表中移除）
- `Achieved` → 不贡献权重（已从 active goals 列表中移除）

### 5.2 目标完成后的涟漪

当一个 goal 完成后，在新 goal 生成之前（下一个 7 天反思周期），有一段时间的"目标空白"：
- intrinsic_motivation_weight → 1.0（等于无目标状态）
- NPC 的行为权重回退到基本需求 + 习惯 + 情绪驱动
- 这可能持续最多 7 天——然后 SelfNarrative 生成新目标

这是**正确且期望的行为**——完成一个重要目标后，NPC 需要一段"消化期"。不是永动机。

---

## 六、与 need_action_match 的权重竞争

### 6.1 权重链中的位置

```
need_action_match(9维 deficit urgency)    → range [1.0, 2.0]
    ×
intrinsic_motivation_weight(goals, action) → range [1.0, 3.0]
    ×
survival_suppression                       → range [0.08, 1.0]
```

两个乘数是**独立且平行**的。这意味着：

- 极端饥饿 (need_action_match × 2.0) + 强烈创作欲 (intrinsic × 3.0) → 组合权重 × 6.0
- 如果此时生存抑制 = 0.12（严重生存危机） → 最终 × 0.72
- 饥饿仍然可能压倒创作欲——因为 survival_suppression 在两者之后作用

### 6.2 为什么不在 need_action_match 和 intrinsic 之间做优先级？

如果我们在架构上说"deficit 优先于 aspiration"，那就成了一个硬规则——一旦饥饿，所有内在目标就失去意义。这是不符合实际的。

更好的方式：让 survival_suppression 作为一个**统一的软阻尼**，在所有高层权重计算完成后统一衰减。这样：
- 轻度饥饿时 intrinsic 几乎不受影响
- 严重饥饿时 intrinsic 被显著抑制
- 但抑制是**连续的**，不是**二值的**

---

## 七、开放问题

1. **ActOnValue 的默认 relevance**：用 0.5 作为保守估计会弱化自主性驱动的行为。是否需要更激进的默认值（0.7）？
2. **goal 数量上限**：当前没有限制——是否应该限制 active goals ≤ 3？防止权重爆炸？
3. **跨 goal 协同**：两个 goal 都推同一个 action 时，是否需要协同 bonus（不只是乘法独立）？
4. **relevance 缓存**：relevance 是纯函数，可以预计算成 lookup table `[GoalType][ActionType] -> f32`——是否应该？

---

> **下一步**: [[文件备份/20260618/开发阶段/NPC活人感模块/04-进阶需求系统/005-跨层桥接机制]] — 生存抑制(sigmoid) + 挫折回归(ERG) 的完整数学设计
