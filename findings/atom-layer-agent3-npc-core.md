# 代理 3 发现：NPC核心 + 生命周期 + 技能 + 语言表达

## 关键统计
- **60+ 命名行动** · **19 现存原子 + 9 新提议**

## 新原子提议

| 原子 | 来源 | 理由 |
|------|------|------|
| SPEAK | 语言表达/NPC社交 | 语音产生——所有对话/演说/悄悄话的基础 |
| GESTURE | 语言表达非语言 | 手势/身体非语言沟通 |
| SLEEP | 认知/睡眠 | 持续POSTURE+WAIT + 独特认知加工(记忆巩固/情绪重置/L1规则化) |
| VENT | 基本需求-元素平衡 | 内部元素释放，无外部信号 |
| NURSE | 生命周期-哺乳 | 生物喂养转移，不同于EAT/INSERT |
| CRAWL | 生命周期-婴儿 | 原始MOVE变体 |
| DIGEST | 基本需求 | 被动物质消耗的内部处理 |
| OBSERVE | 审美/感知 | 持续的感知注意力（不含动作） |
| BEAR/CARRY | 生命周期-育儿 | 持续LIFT+GRASP+MOVE 用于携带活体 |

## 生命周期连续参数曲线（零门控）

所有参数连续演化——无 `if age < X { return false }`：

- fertility_potential: 0→sigmoid上升→峰值→sigmoid下降
- learning_rate: 童年最高→青春期高→成人基线→老年连续下降
- planning_horizon (GOAP): 1(婴儿)→随认知发展增长→5-7(成人)→老年可能收缩
- physical_strength: 低→快速生长→峰值(Adult)→连续下降(Elder)
- complexity_tolerance: 10→18上升→45岁峰值→0.008/年下降
- Gompertz mortality: age_pct 0.7后指数加速

## 关键涌现链

### 求偶链（不脚本化，从atom+社交需求涌现）
SeekProximity → DisplaySkill → GiveGift → VerbalCompliment → PhysicalTouch → ProposeBond

### 认知→书写→历史痕迹链
ThoughtFragment(SurfacingModality::WritingImpulse) → Write(GRASP+SPREAD) → PhysicalBook → HistorySystem → LifeTrace → 大日志

### 审美链
Perceive(AestheticSignal) → Judge(纯函数) → React(情绪/记忆/行为) → Articulate(表达) → Adopt(品味传播) → Embellish(创作)
