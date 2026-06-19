# 006-Blueprint 蓝图系统

> **开发代号**: WoWorld (Wonder World)
> **创建日期**: 2026-06-19 | **状态**: ✅ v1.0
> **所属模块**: [[建筑模块/README|建筑模块]]

---

## 一、定位

Blueprint 是**玩家的设计文件**——不是系统的内部 IR。它：
- 用于玩家从零建造新建筑、或改造既有建筑
- 可跨存档分享（TOML 文本文件）
- 可作为物品存在背包里（ItemDefId 0x52）
- NPC 的 CONSTRUCT 自发改造不需要 Blueprint——直接操作 ComponentInstance

Blueprint 描述**建筑意图**（什么组件在哪），不描述**建筑实现**（最终质量取决于材料可用性、NPC 技能、施工时的条件）。

---

## 二、TOML 格式

### 2.1 完整示例——三层商住楼

```toml
[meta]
name = "三层商住楼"
version = "1.0"
author = "玩家名"
cultural_hint = "WesternMaritime"        # 偏好提示——非强制。跨世界导入时自动本地化
terrain_tolerance = { max_slope = 10.0, allow_flatten = true }
grid = { main = 1.0, sub = 0.25 }
floors = 3

# ━━ 地面层：店面 ━━
[floor.0]
ceiling_height = 3.5

[floor.0.components.storefront_north]
family = "Wall"
origin = [0, 0, 0]                      # [x, y, z]——y = 离地高度
params = { width = 8.0, height = 3.5, thickness = 0.5, material = "Stone", wall_type = "Exterior", style = "BlockSmooth" }

[floor.0.components.show_window]
family = "Window"
origin = [2, 0, 0]
params = { width = 2.0, height = 1.5, material = "Glass", window_type = "Wide" }

[floor.0.components.main_door]
family = "Door"
origin = [4, 0, 0]
params = { width = 1.5, height = 3.0, material = "Oak", door_type = "Double", swing = "Inward" }

[floor.0.components.floor_slab]
family = "Floor"
origin = [0, 0, 0]
params = { width = 8.0, depth = 6.0, thickness = 0.3, material = "Stone", floor_type = "GroundFloor" }

[floor.0.components.stairs_up]
family = "Stairs"
origin = [6, 0, 3]
params = { floor_from = 0, floor_to = 1, width = 1.2, material = "Oak", stair_type = "Straight" }

# ━━ 二层：住宅 ━━
[floor.1]
ceiling_height = 2.8
# ... 组件 ...

# ━━ 屋顶 ━━
[roof]
style = "Gable"
pitch = 30.0
material = "ClayTile"
overhang = 0.6

# ━━ 子蓝图引用（可选） ━━
[[fragments]]
path = "fragments/gothic_window_set.toml"
origin = [1, 2, 0]
rotation = 0

# ━━ 轻量建议（NPC 可覆盖） ━━
[[suggestions]]
family = "Table"
origin = [3, 0, -3]
params = { material = "Oak" }
importance = "Prefer"                    # Prefer | Optional
```

### 2.2 核心设计决策

**一：连接关系自动推导，不存储在 Blueprint 中。**

Blueprint = 纯组件列表 + 位置。组件的连接关系从几何和 ConnectionFace 兼容性自动推导——玩家不需要手写「这面墙连着那扇门」。

**二：`params` 是族参数，尺寸在 params 内——不分离 `dimensions` 字段。**

```toml
# ✅ 对：所有参数统一在 params 里
params = { width = 1.0, height = 2.1, material = "Oak", swing = "Inward" }

# ❌ 错：分离 dimensions 和 params
dimensions = { width = 1.0, height = 2.1 }
params = { material = "Oak", swing = "Inward" }
```

`dimensions()` 是 `ComponentParams` trait 的方法，从 params 内部提取。Blueprint TOML 不需要区分。

**三：材料字段支持意图标签——不强制写死材料名。**

```toml
params = { material = "Oak" }       # 偏好橡木
params = { material = "!Granite" }  # 硬性要求花岗岩——不可替代
params = { material = "Hardwood" }  # 意图标签——运行时解析为本地可用的硬木物种
```

`!` 前缀 → 硬约束（材料不存在则施工阻塞，等待贸易）。无 `!` → 偏好（NPC 可自行替代近似材料）。

**四：`cultural_hint` 是偏好提示——跨世界导入时自动本地化。**

玩家分享 Blueprint 给朋友。朋友的世界里没有「西方谦逊」文化 → Blueprint 自动适配本地文化参数。布局不变，但材质、颜色、屋顶风格本地化。

**五：子蓝图引用（fragments）——可复用设计片段。**

玩家设计了好看的窗组合 → 保存为 `gothic_window_set.toml`。引用时指定位置和旋转。加载时递归展开（最多 3 层递归深度）。

**六：轻量建议（suggestions）——NPC 可以覆盖。**

放置 Blueprint 施工时，NPC 不强制照搬 suggestions 中的家具。Ta 可能没有这个家具、可能不需要、可能有更好的。

---

## 三、Blueprint 作为物品

`Blueprint` item（ItemDefId 0x52）在 [[物品系统]] 中定义，但其**内容**（TOML 结构和验证规则）由建筑模块定义。

```rust
/// Blueprint 物品的内容——建筑模块定义此 schema
pub struct BlueprintItemContent {
    pub toml_content: String,          // 完整 TOML
    pub hash: [u8; 32],                // SHA-256——用于去重和引用
    pub meta: BlueprintMeta,           // 解析后的元数据（用于背包内预览）
}

pub struct BlueprintMeta {
    pub name: String,
    pub floor_count: u8,
    pub total_area: f32,
    pub component_count: u32,
    pub estimated_materials: MaterialRequirementList,
    pub cultural_hint: Option<String>,
}
```

---

## 四、Blueprint 验证流程

```rust
// 1. 加载 TOML → Blueprint struct
let bp = Blueprint::from_toml(&toml_string)?;

// 2. 验证
let validator = BlueprintValidator { registry: &registry };
let validated = validator.validate(&bp, &build_context)?;

// 3. 生成施工项目
let project = ConstructionProject::from_blueprint(&validated, &build_context);

// 4. 调度施工
scheduler.submit(project);
```

验证步骤（见 [[建筑模块/004-约束求解系统#四、BlueprintValidator|004 §四]]）：
1. 组件族名有效性
2. 参数 schema 验证
3. 尺寸合理性
4. 连接面兼容性（几何自推导）
5. 结构完整性
6. 网格对齐
7. 组件族可用性（`is_available()`）

---

## 五、跨文化移植机制

```rust
impl Blueprint {
    /// 适配到目标文化的参数
    pub fn localize(&self, target_ctx: &BuildContext) -> Blueprint {
        let mut localized = self.clone();

        for component in &mut localized.components {
            let family = REGISTRY.resolve_by_name(&component.family_name).unwrap();

            // 用目标文化的参数覆盖材质和样式
            let default_params = family.default_params(target_ctx);
            component.params.override_with(&default_params);
            // 保留组件位置和结构尺寸（宽高深）
            // 替换材质、样式、颜色
        }

        // 屋顶风格也本地化
        if let Some(culture) = &target_ctx.culture {
            localized.roof.style = culture.roof_style.unwrap_or(localized.roof.style);
        }

        localized
    }
}
```

---

## 六、Blueprint 版本演进

Blueprint TOML 带有 `version` 字段。格式迭代时通过迁移器链式升级：

```rust
trait BlueprintMigrator {
    fn migrate(&self, bp: Value) -> Result<Value, MigrationError>;
    fn source_version(&self) -> &str;
    fn target_version(&self) -> &str;
}

// 注册表: "1.0" → "1.1" → "2.0"
// 加载旧版本 → 自动通过链式迁移升级并提示玩家存为新版本
```

---

## 七、关联文档

- [[建筑模块/001-组件族定义与注册|组件族定义与注册]]
- [[建筑模块/004-约束求解系统|约束求解系统]] — BlueprintValidator
- [[建筑模块/005-施工调度系统|施工调度系统]]
- [[建筑模块/008-跨模块接口与数据合同|跨模块接口与数据合同]]
- [[物品系统/002-物品分类与ID体系|物品分类与 ID 体系]] — `Blueprint` 0x52
