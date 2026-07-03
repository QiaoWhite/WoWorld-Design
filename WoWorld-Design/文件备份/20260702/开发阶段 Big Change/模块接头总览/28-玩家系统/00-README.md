# 28-玩家系统 — 模块接头总览

> **模块**: 玩家系统
> **位置**: `开发阶段/玩家系统/`
> **版本**: v1.0
> **创建日期**: 2026-06-24
> **CHG**: [[../../../Change/CHG-063-玩家系统新建-20260624|CHG-063]]

---

## 模块定位

玩家系统是独立一级模块——定义玩家如何与共享系统交互。不定义新实体类型（Player = SapientMind + ControlMode）。

## 导出概念

| 概念 | Owner | 核心消费方 |
|------|-------|-----------|
| ControlMode (Auto/Manual/DomainDelegated) | 玩家系统 003 | NPC生命周期 008 |
| ActionDomain (6 种) | 玩家系统 003 | 输入系统 |
| PlayerGoal | 玩家系统 004 | NPC活人感 GOAP |
| 两种进入模式（原住民/穿越者） | 玩家系统 001 | 角色创建流程 |
| 信息展示边界 | 玩家系统 006 | UI/UX 系统 |
| 死亡继承链 | 玩家系统 005 | NPC生命周期 007 |
| 输入动作清单（~40 动作） | 玩家系统 006 | UI/UX 系统 |

## 消费的外部概念

| 概念 | Owner 模块 | 消费位置 |
|------|-----------|----------|
| LifeEntity / MindLayer | 生命 001 | 001/002/005 |
| SapientMind / BigFive / CognitiveStyle | NPC活人感 | 001/002/004 |
| GOAP 规划器 | NPC活人感 | 003/004 |
| DeathCause | 生命 004 | 005 |
| SkillEntry / MentalAccess / PhysicalAccess | 技能系统 | 002/004/006 |
| SpellId / 十元素亲和 | 魔法系统 | 002 |
| LanguageId | 语言表达 | 002 |
| PlayerInput | 语言表达 | 006 |
| GrandJournal | 历史 005 | 003/004/005 |
| FairyCompanion | 小精灵系统 | 004 |
| LODCoordinator | 技术栈方案 | 006 |
| SaveableModule | 存档系统 | 005 |
