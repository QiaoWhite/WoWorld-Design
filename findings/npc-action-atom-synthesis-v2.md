# NPC 物理原子层 v2.0 — 综合设计文档

> 基于 5 个并行探索代理遍历全部 20+ 模块（~100,000 行设计文档）的综合产出
> 日期: 2026-06-18
> 覆盖文件: ~170 文件

---

## 一、架构总览：三层原子模型

```
┌─────────────────────────────────────────────────────────────┐
│  第三层: 社会抽象原子 (Social Abstract Atoms)                │
│  ~25 个 — 社交/经济/权力/文化/信仰领域的复合行为单元           │
│  直接承载玩家可见的社会意义，由领域模块注册为 ActionCandidate   │
│  例: TRADE, JUDGE, PRAY, NEGOTIATE, TEACH, CONVERT           │
└───────────────┬─────────────────────────────────────────────┘
                │ 内部实现为: 第二层原子的序列
┌───────────────▼─────────────────────────────────────────────┐
│  第二层: 领域复合原子 (Domain Composite Atoms)               │
│  ~40 个 — 跨多个物理原子的有意义的动作组合                     │
│  由领域模块实现 execute()，消费物理原子                        │
│  例: CRAFT(Smithing)=HEAT+STRIKE+COOL+DIP                    │
│       HARVEST=GRASP+CUT+SCOOP+STACK                          │
└───────────────┬─────────────────────────────────────────────┘
                │ 内部实现为: 第一层原子的序列
┌───────────────▼─────────────────────────────────────────────┐
│  第一层: 物理基元 (Physical Primitives)                       │
│  24 个 — 身体与世界的最小可交互单元                            │
│  定义在 woworld_core，SoA 友好批量执行，零领域知识              │
│  例: MOVE, GRASP, STRIKE, THROW, ATTACH, IGNITE              │
└─────────────────────────────────────────────────────────────┘
```

### 核心设计原则

1. **第一层极薄极稳** — ~24 个原子，只做物理计算，不包含任何领域知识
2. **第二层由领域模块实现** — 每个模块定义自己的复合原子（CRAFT, CAST, HARVEST...），内部调用第一层
3. **第三层是 GOAP 可见的 ActionCandidate** — NPC 规划器操作这一层
4. **年龄/状态从不设门控** — AgentSnapshot 连续参数贯穿全部三层
5. **所有原子自动产生: 声音(音频系统) + 感官事件(SpatialEventBus) + 历史痕迹(History)**

---

## 二、第一层：物理基元（24 个）

### 身体力学 (4)
| # | 原子 | 签名 | 音频 | 感官事件 | 材料属性依赖 |
|---|------|------|------|---------|------------|
| 1 | **MOVE** | `(type, target, speed, gait) → MoveState` | Step声, 表面材质 | 视觉(运动), 听觉(脚步声) | 摩擦力, 坡度, 表面材质, 介质(空气/水) |
| 2 | **POSTURE** | `(pose, duration, blend_ms)` | 衣物摩擦声(微弱) | 视觉(姿态变化) | 表面支撑力 |
| 3 | **JUMP** | `(dir, force, landing_pose) → JumpResult` | 起跳+着陆声 | 视觉+听觉 | 重力, 着陆面硬度 |
| 4 | **WAIT** | `(duration, alertness)` | 无 | 时间流逝(习惯化) | 无 |

### 物体操作 (7)
| # | 原子 | 签名 | 音频 | 感官事件 | 材料属性依赖 |
|---|------|------|------|---------|------------|
| 5 | **GRASP** | `(entity, hand, grip) → GripHandle` | 抓取声 | 视觉(手持物出现) | 重量, 表面摩擦力, 体积 |
| 6 | **RELEASE** | `(grip, mode) → ReleaseResult` | 脱落/放置声 | 视觉+听觉 | 重量→冲击声, 易碎性 |
| 7 | **STRIKE** | `(grip, target, force, angle, contact) → StrikeResult` | 撞击声(最响) | 视觉+听觉+触觉(反震) | 双方硬度/韧性/弹性 |
| 8 | **THROW** | `(grip, dir, force, spin) → FlightResult` | 飞过声+撞击声 | 视觉(弹道)+听觉 | 重量, 空气阻力, 目标材质 |
| 9 | **PUSH** | `(entity, dir, force, contact) → PushResult` | 摩擦/滑动声 | 视觉+听觉 | 摩擦力, 重量, 表面材质 |
| 10 | **PULL** | `(entity, dir, force, grip) → PullResult` | 摩擦/拉拽声 | 视觉+听觉 | 摩擦力, 重量 |
| 11 | **LIFT** | `(grip, height, duration) → LiftResult` | 用力声 | 视觉(高度变化) | 重量, 重心位置 |

### 表面与物质 (6)
| # | 原子 | 签名 | 音频 | 感官事件 | 材料属性依赖 |
|---|------|------|------|---------|------------|
| 12 | **DIP** | `(grip, surface, duration, coverage) → DipResult` | 浸入声 | 视觉+触觉(湿润) | 液体粘度, 物质属性转移 |
| 13 | **POUR** | `(src, target, amount, spread) → PourResult` | 流动声 | 视觉+听觉 | 液体粘度, 扩散率 |
| 14 | **SCOOP** | `(container, source, amount) → ScoopResult` | 舀取声 | 视觉 | 物质颗粒度/粘度 |
| 15 | **SPREAD** | `(substance, surface, area, thickness) → SpreadResult` | 涂抹/摩擦声 | 视觉+嗅觉 | 物质粘度, 表面孔隙率 |
| 16 | **WIPE** | `(surface, implement) → WipeResult` | 擦拭声 | 视觉(清洁) | 表面材质, 摩擦系数 |
| 17 | **ROTATE** | `(grip, axis, angle) → RotateResult` | 转动/机械声 | 视觉(旋转) | 重量, 摩擦力, 转动惯量 |

### 组合与装配 (5)
| # | 原子 | 签名 | 音频 | 感官事件 | 材料属性依赖 |
|---|------|------|------|---------|------------|
| 18 | **ATTACH** | `(a, b, point, bond_type) → CompoundId` | 连接/机械声 | 视觉(物体连接) | 双方材质, bond_type(Rigid/Hinge/Chain/Flexible) |
| 19 | **DETACH** | `(compound, attachment_id) → DetachResult` | 分离声 | 视觉+听觉 | 连接强度, 破坏力 |
| 20 | **STACK** | `(a, b) → StackResult` | 放置/撞击声 | 视觉(堆叠) | 重量, 摩擦力, 重心 |
| 21 | **INSERT** | `(entity, container, tolerance) → InsertResult` | 插入/卡入声 | 视觉(物体消失/嵌入) | 尺寸容差, 摩擦力 |
| 22 | **WEDGE** | `(entity, gap, force) → WedgeResult` | 楔入/挤压声 | 视觉+听觉 | 硬度, 摩擦力, 间隙宽度 |

### 破坏与改变 (3)
| # | 原子 | 签名 | 音频 | 感官事件 | 材料属性依赖 |
|---|------|------|------|---------|------------|
| 23 | **BREAK** | `(entity, force, point) → FractureResult` | 碎裂/断裂声(响亮) | 视觉(碎片)+听觉 | 韧性, 硬度, 脆性 |
| 24 | **CUT** | `(entity, plane, sharpness, force) → CutResult` | 切割声 | 视觉+听觉 | 硬度差(工具vs目标), 韧性 |
| 25 | **CRUSH** | `(entity, force, area) → CrushResult` | 压碎声 | 视觉+听觉 | 抗压强度, 脆性 |

### 能量与元素 (7)
| # | 原子 | 签名 | 音频 | 感官事件 | 材料属性依赖 |
|---|------|------|------|---------|------------|
| 26 | **IGNITE** | `(target, source, intensity) → IgniteResult` | 点燃/爆燃声 | 视觉(火焰)+嗅觉(烟) | 可燃性, 燃点, 湿度 |
| 27 | **EXTINGUISH** | `(target, method) → ExtinguishResult` | 嘶嘶/蒸汽声 | 视觉+嗅觉(烟散) | 方法(水/隔绝/耗尽) |
| 28 | **HEAT** | `(target, source, intensity, duration) → HeatResult` | 沸腾/嘶嘶声 | 视觉(变红/蒸汽)+触觉 | 比热容, 热传导率, 熔点 |
| 29 | **COOL** | `(target, source, intensity) → CoolResult` | 凝结/脆化声 | 视觉(结冰/凝霜) | 比热容, 热传导率, 冰点 |
| 30 | **WET** | `(target, liquid, amount) → WetResult` | 滴水/湿润声 | 视觉(湿润)+嗅觉 | 亲水性, 吸收率 |
| 31 | **CONDUCT** | `(source, target, energy_type) → ConductResult` | 传导嗡鸣(微弱) | 视觉(电弧/能量流) | 电导率, 魔导率, 热导率 |
| 32 | **VENT** | `(element_type, amount) → VentResult` | 微弱释放声 | 无(内部释放) | 元素类型, 元素过剩量 |

### 感知与认知 (3)
| # | 原子 | 签名 | 音频 | 感官事件 | 材料属性依赖 |
|---|------|------|------|---------|------------|
| 33 | **OBSERVE** | `(direction, focus, duration) → ObserveResult` | 无 | 产生PerceptEntry(输入感官系统) | 光线, 距离, 遮挡 |
| 34 | **LISTEN** | `(direction, filter) → ListenResult` | 无 | 输入AudioQuery | 环境噪音, 距离 |
| 35 | **SENSE_AETHER** | `(location, depth) → AetherPerception` | 无(超自然) | 灵元素印记感知 | 树龄, 印记清晰度 |

> **与最初 19 个原子的变化**:
> - 新增: VENT, OBSERVE, LISTEN, SENSE_AETHER, CONDUCT (重新定义为第32号), WET (重新定义)
> - 合并: SOAR/GLIDE/HOVER → MOVE 的 locomotion_type 参数
> - 合并: PROPEL → MOVE 的介质参数
> - CRAWL → MOVE 的 locomotion_type::Crawl 变体
> - 保留 POSTURE 但新增 KNEEL/BOW/PROSTRATE 作为 POSTURE 的 pose 参数变体（文化意义太重，不独立为物理原子）

---

## 三、第二层：领域复合原子（~40 个）

各领域模块在其 `execute()` 内部组合第一层物理原子。

### 3.1 建造与生产 (8)
| 复合原子 | 物理原子序列 | Owner 模块 | 关键材料属性 |
|---------|------------|-----------|------------|
| **CRAFT** | GRASP+HEAT+STRIKE+COOL+DIP | 经济/物品 | 金属熔点, 淬火介质 |
| **HARVEST** | GRASP+CUT/SCOOP+STACK | 生命/经济 | 植物成熟度, 工具锋利度 |
| **DIG** | STRIKE+BREAK+SCOOP+LIFT | 世界生成/经济 | 土壤硬度, 工具等级 |
| **FELL** | STRIKE+CUT+PUSH(定向) | 生命/经济 | 木材硬度, 树径 |
| **CONSTRUCT** | LIFT+ATTACH+STRIKE+SPREAD | 建造/文化 | 建材承重, 连接强度 |
| **PLANT** | SCOOP+INSERT+SPREAD(水)+WET | 生命/经济 | 土壤肥力, 种子品质 |
| **COOK** | CUT+POUR+HEAT+SCOOP+MIX | 经济/基本需求 | 食材新鲜度, 火候 |
| **MINE** | MOVE+STRIKE+BREAK+LIFT+SCOOP | 经济 | 矿石硬度, 储量 |

### 3.2 战斗 (5)
| 复合原子 | 物理原子序列 | Owner 模块 | 关键参数 |
|---------|------------|-----------|---------|
| **PARRY** | MOVE(微调)+STRIKE(偏转) | 战斗 | 时机窗口0.1-0.2s |
| **GRAPPL** | GRASP(活体)+PULL+PUSH+THROW | 战斗 | 力量对抗, 体重差 |
| **FEINT** | STRIKE(假)+MOVE(收回)+STRIKE(真) | 战斗 | 欺骗值, 对方感知 |
| **AIM** | POSTURE+OBSERVE+WAIT | 战斗(远程) | 专注力, 风速 |
| **DODGE** | MOVE(侧移)+JUMP+POSTURE(闪避) | 战斗(本能层) | 敏捷, 时机 |

### 3.3 魔法 (6)
| 复合原子 | 物理原子序列 | Owner 模块 | 关键参数 |
|---------|------------|-----------|---------|
| **CAST** | POSTURE+CHANT+GRASP(法器)+ROTATE | 魔法 | 法力消耗, 元素亲和 |
| **CHANT** | 持续WAIT+发声(无物理原子，走音频管道) | 魔法 | 咏唱时长, 打断风险 |
| **CHANNEL** | POSTURE+WAIT(持续)+CONDUCT(魔力输出) | 魔法/载具 | 魔力消耗速率 |
| **TRANSMUTE** | CONDUCT(元素注入)+HEAT/COOL | 魔法(变幻系) | 元素亲和, 目标材质 |
| **INFUSE** | CONDUCT+ATTACH(魔力→物品) | 魔法(附魔) | 魔导率, 附魔容量 |
| **BIND** | ATTACH(精神连接)+CONDUCT(灵魂桥) | 魔法(契约) | 精神强度, 契约代价 |

### 3.4 社交与文化 (9)
| 复合原子 | 物理原子序列 | Owner 模块 | 关键参数 |
|---------|------------|-----------|---------|
| **SPEAK** | POSTURE+WAIT+OBSERVE(反馈) | 语言表达 | 音量, 清晰度, 语言 |
| **GESTURE** | MOVE(手/臂)+POSTURE(上身) | 语言表达/文化 | 文化编码映射 |
| **KNEEL** | POSTURE(Kneel pose)+WAIT | 权力/信仰/文化 | 深度, 持续时间 |
| **BOW** | POSTURE(Bow pose, depth) | 文化(问候) | 深度=权力距离编码 |
| **PROCESSION** | MOVE(集体路径)+POSTURE+GRASP(仪仗) | 文化(节日/葬礼) | 路径, 队形 |
| **DANCE** | MOVE+JUMP+ROTATE+POSTURE(序列) | 文化(节日/仪式) | 节奏, 复杂度 |
| **PRAY** | POSTURE+GRASP(圣物)+CHANT(祷词) | 信仰/文化 | 虔诚度, 神殿位置 |
| **SACRIFICE** | GRASP+POUR/CUT/IGNITE(祭品) | 信仰/文化 | 祭品价值, 仪式规范 |
| **MEDITATE** | POSTURE+WAIT(长时)+VENT(精神) | 信仰/认知 | 神秘主义参数 |

### 3.5 经济与社会交换 (6)
| 复合原子 | 物理原子序列 | Owner 模块 | 关键参数 |
|---------|------------|-----------|---------|
| **EXCHANGE** | GRASP+RELEASE(A→B)+GRASP+RELEASE(B→A) | 经济 | 物品价值, 双方信任 |
| **DISPLAY** | GRASP+STACK+PUSH(展示位置) | 经济(店铺) | 物品外观, 光照明 |
| **SIGN** | GRASP(笔)+SPREAD(墨)+ATTACH(印章) | 权力/经济 | 文档效力, 印章权威 |
| **SEAL** | ATTACH(封蜡)+PUSH(印章) | 权力(文书) | 印章来源, 封装完整性 |
| **PROCLAIM** | POSTURE+MOVE(公示点)+ATTACH(公告)+SPEAK | 权力(立法) | 公告范围, 合法性 |
| **INVEST** | GRASP(权标)+ATTACH(授予)+POSTURE(接受跪姿) | 权力(授权) | 权标象征, 仪式完整 |

### 3.6 生命与生理 (6)
| 复合原子 | 物理原子序列 | Owner 模块 | 关键参数 |
|---------|------------|-----------|---------|
| **SLEEP** | POSTURE(卧姿)+WAIT(长时)+VENT(认知加工) | 生命/认知 | 持续时长, 深度 |
| **NURSE** | GRASP(婴儿)+POSTURE+WAIT(喂食) | 生命周期 | 乳汁供应, 婴儿需求 |
| **INGEST** | GRASP+INSERT(口)+CRUSH(咀嚼)+WET(吞咽) | 基本需求 | 食物元素属性, 消化速率 |
| **BURROW** | MOVE(地下)+BREAK+SCOOP | 生命(动物) | 土壤类型, 挖掘能力 |
| **DORM** | POSTURE+WAIT(超长时)+VENT(代谢关闭) | 生命(冬眠) | 脂肪储备, 环境温度 |
| **MORPH** | WAIT(蜕变期, 身体重构) | 生命(变态发育) | 蜕变时长, 环境触发 |

### 3.7 感知与认知 (4)
| 复合原子 | 物理原子序列 | Owner 模块 | 关键参数 |
|---------|------------|-----------|---------|
| **EXAMINE** | GRASP+ROTATE+OBSERVE(近距离) | NPC/物品 | 物品属性, 感知技能 |
| **READ** | GRASP(书)+OBSERVE(文字)+WAIT | 语言/认知 | 识字率, 文本复杂度 |
| **WRITE** | GRASP(笔)+SPREAD(墨)+OBSERVE(校验) | 语言/历史 | 书法技能, 材料质量 |
| **TRACK** | MOVE(跟踪)+OBSERVE(足迹)+LISTEN | 生命(狩猎) | 地面类型, 天气 |

---

## 四、第三层：社会抽象原子（GOAP 可见，~25 个）

这些是社会意义层面的行为单元——NPC 的 GOAP 规划器直接操作这一层。

| 域 | 抽象原子 |
|----|---------|
| **生存** | Eat, Drink, Sleep, Rest, SeekSafety, VentElements |
| **工作** | Work(根据ProfessionTag分发到CRAFT/HARVEST/MINE/...), GatherResources |
| **经济** | TRADE(Buy/Sell), BrowseShop, OpenStorefront, PlaceOrder |
| **社交** | Socialize, SeekPartner, AttendFestival, GiveGift, Gossip |
| **权力** | JUDGE, LEGISLATE, SANCTION, CollectTax, EnforceLaw, DECLARE_WAR |
| **信仰** | Worship, PRAY, PILGRIMAGE, SACRIFICE, Convert, ORDAIN |
| **文化** | PERFORM(DANCE/MUSIC), FEAST, CelebrateFestival, Mourn, Marry |
| **学习** | LearnSkill, Teach, ReadBook, MEDITATE, Investigate |
| **战斗** | Fight, Defend, Flee, Ambush, Command |
| **创造** | CRAFT, Build, Embellish, COMPOSE |
| **育儿** | Nurse, CarryInfant, RaiseYoung |

**关键**：GOAP 不硬编码这个列表。这只是当前所有模块注册表的人读归纳。新模块 = 新注册 = GOAP 自动可见。

---

## 五、AgentSnapshot — 贯穿全三层的连续参数快照

```rust
struct AgentSnapshot {  // ~128 bytes, SoA友好
    // === 身体 (从 Life + 生命周期 + 身体状态 派生) ===
    strength: f32,           // 0-1 归一化 → GRASP/LIFT/STRIKE力度
    dexterity: f32,          // 0-1 → DODGE/THROW精度
    stamina: f32,            // 0-1 → MOVE持续/STRIKE频率
    health_penalty: f32,     // 0-1 中毒/受伤/疾病累积 → 全原子效率×系数
    mobility_factor: f32,    // 0-1 年龄+体型+负重 → MOVE速度/可达性
    body_temp: f32,          // K → COOL/HEAT耐受
    fatigue: f32,            // 0-1 → WAIT恢复速率
    hunger: f32,             // 0-1 → GOAP eat目标权重
    
    // === 认知 (从 认知系统+NPC人格+技能 派生) ===
    skill_vector_compressed: u64,     // 压缩技能向量
    mental_model_count: u8,           // 幼儿<成人
    cognitive_damping: f32,           // 0-1 认知僵化 → 学习新概念速率
    planning_horizon: u8,             // GOAP搜索深度(幼儿1→成人5-7→老年可能收缩)
    attention_capacity: f32,          // 0-1 → OBSERVE/LISTEN能效
    literacy: f32,                    // 0-1 → READ/WRITE可行性
    financial_literacy: f32,          // 0-1 → 交易精度
    
    // === 社交 (从 权力+名声+文化+关系 派生) ===
    legal_capacity: f32,              // 0-1 法律行为能力 → SIGN/INVEST/PROCLAIM
    reputation_flags: u32,            // 压缩声望
    social_standing: f32,             // 0-1 → 说服力/交涉成功率
    religious_participation: f32,     // 0-1 → PRAY/SACRIFICE效能
    
    // === 生命阶段 (从 生命周期 CHG-041 派生) ===
    developmental_phase: u8,          // Infant/Toddler/Child/Adolescent/YoungAdult/Adult/MiddleAge/Elder
    age_ratio_in_phase: f32,          // 0-1 当前阶段内位置
    fertility_potential: f32,         // 0-1 sigmoid曲线
    gompertz_mortality_risk: f32,     // 当前死亡风险
    
    // === 环境 (从 天气+感官 派生) ===
    wetness: f32,                     // → 摩擦力/导电性/可燃性修正
    temperature_exposure: f32,        // → HEAT/COOL耐受偏移
    intoxication: f32,                // 0-1 → 全原子精度×噪声
}
```

---

## 六、年龄→ActionCandidate 涌现矩阵

**核心理念**：无一行代码说 `if age < X { cannot_do(Y) }`。所有差异从 AgentSnapshot 连续参数涌现。

| 生命阶段 | AgentSnapshot 特征 | 涌现的行为特征 |
|---------|-------------------|-------------|
| **Infant (婴儿)** | strength=0.05, mental_model_count=0, planning_horizon=1, legal_capacity=0, literacy=0, fertility=0, mobility=CRAWL | GOAP只生成cry/seek_mother; GRASP任何东西都失败(太重); 不会形成结构化学习目标; 完全依赖照料者 |
| **Toddler (幼儿)** | strength=0.15, mental_model_count=2-5, planning_horizon=1-2, mobility=WALK(slow) | 开始GRASP小物体; 模仿成年人POSTURE; 玩耍涌现(低风险探索); 不能CRAFT(物理参数太低) |
| **Child (儿童)** | strength=0.3, learning_rate=MAX, planning_horizon=3, literacy=emerging | 最高学习速率; 开始简单CRAFT(学徒); PLAY主导行为; READ/WRITE取决于教育; 不能INVEST/SIGN(legal_capacity=0); 不能Marry(fertility≈0) |
| **Adolescent (少年)** | strength=0.6, fertility=rising, aesthetic_openness=MAX, planning_horizon=4 | AestheticTaste开始结晶; Courtship行为出现; 社交从Play转向Status; 身体接近成年但skill低; 反叛因素影响Authority服从 |
| **YoungAdult (青年)** | strength=0.9, fertility=PEAK, planning_horizon=5-6, mobility=PEAK | 全原子能力巅峰; 长途旅行(军事/贸易/朝圣); 核心婚育期; 职业确立期 |
| **Adult (成年)** | strength=0.95, cognitive_damping=基线, skill_vector=丰富 | 全原子能力维持; 从个体竞争转向团体协作; 权力角色承担(INVEST/JUDGE/PROCLAIM); 文化传承(Teach) |
| **MiddleAge (中年)** | strength=0.85(开始缓慢下降), aesthetic_complexity=PEAK | 身体开始慢速退化(连续,非门控); 审美复杂度达峰→艺术创作质量最高; 社会资本累积→更频繁使用权力原子 |
| **Elder (老年)** | strength=0.6→0.3(加速下降), gompertz=指数上升, cognitive_damping=升高, mobility=慢, fertility→0 | STRIKE力量不足→转向轻工具/策略; MOVE(跑步)→MOVE(步行); 从生产者→导师/裁判/祭司(JUDGE/TEACH/PRAY权重↑); 临终整理(WriteLifeTrace); 哀悼丧偶(Mourn涌现) |

**关键涌现链**：
- 儿童不能锻造 → 不是因为"儿童不能打铁" → 是因为 strength=0.3 无法满足 GRASP(大锤) 的最小力量要求
- 老人不能打仗 → 不是因为"老人不能参军" → 是因为 strength连续下降 + stamina恢复率低 → 战斗评估失败
- 中毒者不劳动 → 不是因为"中毒不能打工" → 是因为 health_penalty → 全原子效用×下降 → GOAP选择买药 > 打工

---

## 七、MaterialProperties — 涌现的真正引擎

```rust
struct MaterialProperties {  // 每个物理实体都有此表
    // 力学 (影响: GRASP/STRIKE/BREAK/LIFT/PUSH/PULL/ATTACH/CRUSH/CUT)
    density: f32,              // kg/m³ → 重量、浮力、惯性
    hardness: f32,             // 0-1 Mohs式 → "谁划伤谁"
    toughness: f32,            // 0-1 → 抗断裂(BREAK阈值)
    elasticity: f32,           // 0-1 → 弹性碰撞vs塑性变形
    friction: f32,             // 0-1 → GRASP/STACK稳定性、MOVE抓地
    
    // 热学 (影响: HEAT/COOL/IGNITE/EXTINGUISH)
    specific_heat: f32,        // 升温所需能量
    thermal_conductivity: f32, // 传热快慢
    melting_point: f32,        // K → HEAT超过=融化
    ignition_point: f32,       // K → IGNITE阈值
    combustibility: f32,       // 0-1 → IGNITE后燃烧强度
    
    // 电学/魔导 (影响: CONDUCT)
    electrical_conductivity: f32, // 0(绝缘)-1(超导)
    mana_conductivity: f32,       // 0-1 → INFUSE/附魔容量
    
    // 化学/表面 (影响: DIP/POUR/SPREAD/WET/WIPE)
    flammability: f32,         // 0-1
    solubility: f32,           // 0-1 → 水中溶解
    acidity: f32,             // -1(强碱)~1(强酸)
    toxicity: f32,             // 0-1 → INGEST/DIP风险
    porosity: f32,             // 0-1 → SPREAD吸收率、SPREAD附着
    
    // 感官 (影响: OBSERVE/LISTEN的感知质量)
    reflectivity: f32,         // 0-1 → 视觉显著性
    sound_damping: f32,        // 0-1 → 声音衰减
    scent_intensity: f32,      // 0-1 → 嗅觉检测距离
    
    // 魔法 (影响: CAST/INFUSE/TRANSMUTE/SENSE_AETHER)
    elemental_affinity: [f32; 10],  // 对10元素的亲和度
    enchantment_capacity: f32,      // 0-1 → 承载附魔上限
    aether_retention: f32,          // 0-1 → AetherImprint保留效率
}
```

---

## 八、原子→模块消费矩阵

| 模块 | 消费的第一层原子 | 定义的领域复合原子 | 消费的第三层ActionCandidate |
|------|---------------|-----------------|-------------------------|
| NPC活人感 | MOVE,POSTURE,WAIT,OBSERVE | — | Eat,Drink,Sleep,Socialize,Flee |
| 基本需求 | INGEST,VENT,SLEEP | — | Eat,Drink,Sleep,VentElements |
| 进阶需求 | MOVE,POSTURE,OBSERVE | — | SeekRecognition,Compete,ShowOff |
| 审美 | OBSERVE,POSTURE | CRAFT(Embellish) | React,Articulate,Adopt,Embellish |
| 认知 | OBSERVE,READ,WRITE,SENSE_AETHER | MEDITATE | LearnSkill,Teach,ReadBook |
| 生命周期 | MOVE,POSTURE,GRASP,NURSE | SLEEP,DORM,MORPH,NURSE | Nurse,CarryInfant,Die |
| 战斗 | MOVE,STRIKE,THROW,GRASP,PUSH | PARRY,GRAPPL,FEINT,AIM,DODGE | Fight,Defend,Flee,Ambush |
| 魔法 | POSTURE,GRASP,ROTATE,CONDUCT | CAST,CHANT,CHANNEL,TRANSMUTE,INFUSE,BIND | CastSpell,Summon,Enchant |
| 世界生成 | (生成时: ATTACH,CONSTRUCT,STACK) | DIG,FELL,CONSTRUCT | — |
| 生命(动物) | MOVE,POSTURE,JUMP,GRASP,STRIKE,BURROW | TRACK,DORM,BURROW,MORPH | Hunt,Forage,Flee,Mate,Hibernate |
| 生命(植物) | OBSERVE,SENSE_AETHER | PLANT,HARVEST | ReadTree,GatherHerbs |
| 天气 | 无直接消费(修正所有1层原子) | — | — |
| 历史 | OBSERVE(痕迹生成),WRITE,SIGN | WRITE,SIGN | RecordTrace,WriteChronicle |
| 物品 | GRASP,RELEASE,ATTACH,DETACH,STACK,INSERT | CRAFT,EXCHANGE | CraftItem,Equip,Repair |
| 技能 | 领域依赖(复用模块原子) | TEACH | LearnSkill,Teach,TrainSkill |
| 语言表达 | POSTURE,WAIT,OBSERVE,LISTEN | SPEAK,GESTURE | Converse,Gossip,Deceive |
| 经济 | GRASP,RELEASE,EXCHANGE,DISPLAY,PUSH | CRAFT,TRADE,EXCHANGE,DISPLAY | Buy,Sell,Browse,PlaceOrder |
| 权力 | POSTURE,KNEEL,GRASP,PUSH,ATTACH,SIGN | KNEEL,BOW,SIGN,SEAL,PROCLAIM,INVEST | JUDGE,LEGISLATE,SANCTION,CollectTax |
| 文化 | MOVE,POSTURE,GRASP,ATTACH,POUR,STRIKE | DANCE,PROCESSION,FEAST,KNEEL,BOW,GESTURE | CelebrateFestival,Mourn,Marry |
| 信仰 | POSTURE,KNEEL,GRASP,POUR,IGNITE,MEDITATE | PRAY,SACRIFICE,MEDITATE,PROCESSION | Worship,Pray,Pilgrimage,Convert,Ordain |
| 感官 | OBSERVE,LISTEN(被动) | — | Perceive,Scan,Sniff |
| 音频 | (所有物理原子自动产生声音→音频模块消费) | — | — |
| 载具 | MOVE,ROTATE,GRASP,ATTACH | STEER,BOARD,ANCHOR,HOIST | Navigate,Board,Repair,CrewDuty |
| 模型/动画 | MOVE(步态),POSTURE(姿态),所有原子驱动骨骼 | — | — |
| 空间查询 | (被所有1层原子消费，不是消费原子) | — | — |
| 技术栈/性能 | (预算管理，不消费原子) | — | — |

---

## 九、性能架构

### 原子执行的分层策略

| L层 | 距离 | NPC数 | 原子执行 | 频率 |
|-----|------|-------|---------|------|
| L0 | 0-10m | ≤50 | 全原子物理计算 | 每帧(16.7ms) |
| L1 | 10-50m | ≤950 | 全原子物理计算 | 每0.3s(分帧) |
| L2a | 50-100m | ~3000 | 简化原子(跳帧,合并步态) | 每1-5s |
| L2b | 100-200m | ~7000 | 统计行为(不逐原子) | 每5-30s |
| L3-L4 | 200m+ | ~89K+ | LOD协调器: 骨骼LOD+N→原子降级到统计 | 每日/每月 |

### 每帧原子预算

| 原子组 | 预算 | 关键优化 |
|-------|------|---------|
| MOVE(1000 NPC步态) | ≤0.5ms | rayon 6核, 917 NPC链式乘法 |
| STRIKE/GRASP(战斗) | ≤1.5ms | 仅战斗帧; 与GOAP互斥 |
| 感官(OBSERVE/LISTEN) | ≤1.0ms | 分帧17次×0.05ms; 战斗时射线减半 |
| 音频(所有原子产生声) | ≤0.17ms | 持续固定成本 |
| 魔法原子 | ≤0.3ms | 环境交互传播采样 |
| **总计Rust侧** | **≤4.5ms** | 峰值互斥规则: 战斗+GOAP+感官不会同时峰值 |

---

## 十、设计集成建议

### 与现有系统的对接
1. **音频系统已有的 58 个 ActionAtom** → 映射到本设计的 35 个物理原子 + 25 个领域复合原子
2. **20 个 InteractionType** → 直接映射到第一层 STRIKE/PUSH/PULL/CUT 等
3. **空间查询四trait** → 所有第一层原子实现时消费, 不修改 trait 签名
4. **AgentSnapshot** → 不需要新 trait; 实现 `From<&NpcState> for AgentSnapshot` 即可
5. **ActionCandidate 注册表** → 放 `woworld_core`, 各模块在初始化时注册

### 不引入新 crate
所有新增类型放在已有位置:
- 35个物理原子 → `woworld_core::atoms` (已有 ActionAtom 的位置)
- MaterialProperties → `woworld_core::materials` 
- AgentSnapshot → `woworld_core::agent`
- ActionCandidate 注册表 → `woworld_core::action_registry`
