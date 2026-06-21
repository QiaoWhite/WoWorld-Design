# 004 — Wear 维护系统设计

> **日期**: 2026-06-21
> **状态**: 参考文档。转正条件：Godot 4.7 + godot-rust/gdext 支持确认。
> **关联**: [[002-画面渲染管线v4.7修订|002 渲染管线]] · [[../../Happy Game/开发阶段/建筑模块/README|建筑模块]] · [[../../Happy Game/开发阶段/物品系统/README|物品系统]] · [[../../Happy Game/开发阶段/经济系统/README|经济系统]] · [[../../Happy Game/开发阶段/NPC活人感模块/05-审美与艺术/审美与艺术系统概览|审美系统]]

---

## 〇、核心设计

三层做旧 + 维护循环：

```
第一层: AI 生成贴图自带做旧（最廉价——美术资产本身就旧）
第二层: 全局 aging shader uniform（wear_level → saturation/brightness/dirt blend）
第三层: 共享脏污贴图 triplanar 投影（一张 256² · 全世界共用）

维护: wear -= repair_amount（GOAP 目标 + 物理原子复合序列）
经济: 维护材料需求 → Market 订单簿 → 价格涌现
审美: wear → virtuosity↓ + fluency↓ → NPC 审美判断
```

---

## 一、wear_level 状态机

```rust
struct WearState {
    wear_level: f32,           // 0.0=崭新 → 1.0=破败
    last_update_day: u32,
    material_class: MaterialWearClass,
}

enum MaterialWearClass {
    Stone,    // decay=0.0003/日 (几百年才到 1.0)
    Wood,     // decay=0.0006/日
    Metal,    // decay=0.0005/日
    Fabric,   // decay=0.0012/日
    Organic,  // decay=0.0020/日
}

impl WearState {
    fn update(&mut self, current_day: u32, weather: &WeatherSample) {
        let days = current_day - self.last_update_day;
        if days == 0 { return; }
        let mut decay = self.material_class.base_decay() * days as f32;
        if weather.rain_intensity > 0.5 { decay *= 1.5; }
        if weather.is_storm { decay *= 3.0; }
        self.wear_level = (self.wear_level + decay).min(1.0);
        self.last_update_day = current_day;
    }

    fn maintain(&mut self, quality: f32) {
        self.wear_level = (self.wear_level - 0.2 * quality).max(0.0);
    }
}
```

## 二、与已有模块的对接

| 模块 | 对接方式 | 改动量 |
|------|---------|--------|
| **建筑模块** | BuildingRuntime.wear_level（新增 f32 字段）。世界生成初始值——新建=0，古镇=0.2-0.5 | 小 |
| **物品系统** | 已有 ItemState.durability（与 wear 同义——共用 shader uniform 名） | 零——已有 |
| **经济系统** | 维护 GOAP → 查询 maintenance_materials.toml → Market asks/bids → 订单簿 | 零——已有机制 |
| **审美系统** | `virtuosity *= (1-wear×0.6)`、`fluency *= (1-wear×0.4)` | 一行 |
| **物理原子** | 维护 = WIPE + SPREAD + STRIKE + DETACH + ATTACH 复合序列。零新增原子 | 零 |
| **权力系统** | 公共建筑：Polity 所有 → Polity treasury 支付维护 → Duty 指派工人 | 标注接口 |

## 三、wear 与 health 的区别

- `health`（已有·ComponentHot.health）：**结构完整性**——建筑会塌吗。物理。
- `wear`（新增·visual）：**视觉老化**——建筑看起来旧吗。渲染。
- 正交：health=0.3/wear=0.1 = 地震后的新建筑。health=0.9/wear=0.7 = 百年老石塔。

## 四、维护材料映射

```toml
# maintenance_materials.toml
[building.wood_wall]
material = "item:wood_planks"
amount = 3
tool_tags = ["hammer", "saw"]

[item.iron_blade]
material = "item:whetstone"
amount = 1
tool_tags = ["whetstone"]
durability_restored = 0.3
```

已有 ItemRegistry + 配方表 tool_tags 匹配管道——零新代码。

## 五、Shiny 体验

- 极慢衰败（石头 100 年→wear=1.0）。维护 ≈ 每年一次事件——不频繁
- 玩家建造 → wear=0（崭新）→ 随时间流逝慢慢变旧 → 定期维护 → 保持 0.05-0.15
- 委托 NPC 维护（付费）→ NPC 技能决定维护质量 → 涌现的信任关系
- 世界生成时：古镇 = wear 0.2-0.5（有人维护）。废墟 = 0.6-0.9（无主）

## 六、公共建筑维护

建筑所有者=PolityId → Polity treasury 支付 → Duty 指派工人 → GOAP MaintainPublicBuilding。已有的权力系统 Extract + Duty 机制——零新增。

---

> **关联**: [[002-画面渲染管线v4.7修订|002 渲染管线 §六]] · [[../../Happy Game/开发阶段/建筑模块/002-建筑数据模型|建筑数据模型]]
