> **开发代号**: WoWorld (Wonder World)
> **文档类型**: 参考文档 · 讨论大纲
> **创建日期**: 2026-06-17
> **父文档**: [[000-模块大纲与导航]]

---

# 003-MentalModel生命周期

## 待讨论的核心问题

### 1. MentalModel的数据结构

- pattern字段：如何表达"当X发生时Y倾向于跟着发生"？
- confidence：贝叶斯置信度
- supporting_count vs counter_count
- supporting_diversity：防止过度拟合的关键字段
- emotional_charge：情绪附着（EMA累积）
- source：SelfDerived / SelfRefined / TaughtByNpc / ReadFromBook / Inherited
- shareability：Public / InGroup / SpecificTarget / Private
- activation_threshold：惰性遗忘的门槛

### 2. 归纳（try_induce_pattern）

- 从记忆中提取共现模式
- 同domain下≥3条高impact记忆→触发归纳
- 拉普拉斯平滑→初始confidence
- 对标已有记忆系统的MemoryPartition索引

### 3. 微消化（micro_digest）

- 每记忆编码时触发
- 认知偏误调制：确认偏误放大支持证据、负面偏误乘负面证据
- 贝叶斯更新公式
- 情绪附着EMA

### 4. 宏消化（macro_digest）

- 每7天随SelfNarrative::reflect()触发
- source转移：外来→SelfRefined（消化深度>0.8）
- 模型融合：同一domain+结构相似的模型
- 边界扩展：高开放性对高confidence模型附加例外条件
- 放弃：confidence<0.1且counter>>supporting

### 5. 种子继承——NPC出生时脑子里有什么？

- 对标信仰系统ChildFaithProfile
- 父母传递：confidence × 继承因子
- 聚落常识：SettlementWisdomAggregate中高频模型
- 文化基线：文化隐含的世界观（source=Inherited）

### 6. MentalModel冲突与群体认知

- 两个NPC的MentalModel矛盾→辩论/回避/深入探讨
- 群体对话中的集体WisdomSharing→共识涌现/少数意见保留
- 对标已有SocialField（仅从态度扩展到MentalModel）

### 7. 创造性飞跃（creative_leap）

- 前置条件：多种参数组合的门槛
- 跨界结构模板匹配
- 产出：低confidence MentalModel（hypothesis=true）

### 8. Zeigarnik效应与反刍

- 未消化的不确定模型→占用rumination资源
- 高neuroticism→放大效应→反刍循环
