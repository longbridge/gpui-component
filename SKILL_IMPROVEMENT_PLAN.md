# GPUI Component Skills 改进计划

基于 skill-creator 标准的详细分析和改进建议。

## 分析概要

### 统计数据

| Skill 名称 | 行数 | 状态 | 优先级 |
|-----------|------|------|--------|
| gpui-element | 757 | ❌ 严重超标 | P0 |
| gpui-entity | 457 | ❌ 严重超标 | P0 |
| gpui-async | 441 | ❌ 严重超标 | P0 |
| gpui-context | 429 | ❌ 严重超标 | P0 |
| gpui-event | 414 | ❌ 严重超标 | P0 |
| gpui-focus-handle | 393 | ❌ 严重超标 | P0 |
| gpui-global | 300 | ❌ 超标 | P1 |
| gpui-layout-and-style | 194 | ⚠️ 接近上限 | P2 |
| gpui-action | 187 | ⚠️ 接近上限 | P2 |
| gpui-test | 94 | ✅ 良好 | - |
| github-pull-request-description | 44 | ✅ 良好 | - |
| new-component | 22 | ✅ 良好 | - |
| generate-component-documentation | 20 | ✅ 良好 | - |
| generate-component-story | 20 | ✅ 良好 | - |

**标准**: Skill-creator 建议 SKILL.md 保持在 500 行以下，理想情况下更短。

## 核心问题

### 1. 违反简洁性原则

**问题**: 8 个 GPUI 框架相关的 skills 内容过长（187-757 行），严重违反了 skill-creator 的核心原则。

**引用 skill-creator**:
> "Concise is Key: The context window is a public good. Keep SKILL.md body to the essentials and under 500 lines to minimize context bloat."

**影响**:
- 浪费宝贵的上下文窗口
- 每次触发 skill 都加载大量不必要的内容
- 降低 Claude 处理效率

### 2. 缺乏渐进式披露

**问题**: 只有 `gpui-test` 正确使用了渐进式披露（有独立的 reference.md 和 examples.md），其他 GPUI skills 都将所有内容放在 SKILL.md 中。

**引用 skill-creator**:
> "Progressive Disclosure Design Principle: Skills use a three-level loading system:
> 1. Metadata (name + description) - Always in context (~100 words)
> 2. SKILL.md body - When skill triggers (<5k words)
> 3. Bundled resources - As needed by Claude"

**最佳实践**: `gpui-test` 的结构
```
gpui-test/
├── SKILL.md (94 行 - 核心概览和快速入门)
├── reference.md (详细的 API 参考和模式)
└── examples.md (完整的示例代码)
```

### 3. Description 质量不一致

**问题**: 部分 skills 的 description 不够详细，未充分说明触发条件。

**引用 skill-creator**:
> "description: This is the primary triggering mechanism for your skill. Include both what the Skill does and specific triggers/contexts for when to use it."

## 详细改进建议

### P0 优先级 - 立即改进（6 个 skills）

#### 1. gpui-element (757 行 → 目标 <100 行)

**当前问题**:
- 包含大量详细的 API 文档和代码示例
- 没有使用 references/ 目录

**改进方案**:
```
gpui-element/
├── SKILL.md (~80 行)
│   ├── Overview (简要说明何时使用 Element trait vs Render/RenderOnce)
│   ├── Quick Start (最基础的示例)
│   └── 引导到详细文档
├── references/
│   ├── api-reference.md (Element trait 完整 API 文档)
│   ├── layout-phase.md (Request Layout 阶段详解)
│   ├── prepaint-phase.md (Prepaint 阶段详解)
│   ├── paint-phase.md (Paint 阶段详解)
│   └── best-practices.md (最佳实践和性能优化)
└── examples/
    ├── simple-element.md (简单元素示例)
    ├── interactive-element.md (交互元素示例)
    └── complex-element.md (复杂元素示例)
```

**SKILL.md 结构**:
```markdown
---
name: gpui-element
description: Implementing custom elements using GPUI's low-level Element API (vs. high-level Render/RenderOnce APIs). Use when you need maximum control over layout, prepaint, and paint phases for complex custom UI components.
---

## When to Use

Use Element trait when:
- Need fine-grained control over layout calculation
- Building complex, performance-critical components
- High-level Render/RenderOnce APIs are insufficient

For simple components, prefer Render/RenderOnce traits instead.

## Quick Start

[简单示例展示基本结构]

## Reference Documentation

- **API Reference**: See [api-reference.md](references/api-reference.md)
- **Layout Phase**: See [layout-phase.md](references/layout-phase.md)
- **Prepaint Phase**: See [prepaint-phase.md](references/prepaint-phase.md)
- **Paint Phase**: See [paint-phase.md](references/paint-phase.md)
- **Examples**: See examples/ directory
- **Best Practices**: See [best-practices.md](references/best-practices.md)
```

#### 2. gpui-entity (457 行 → 目标 <100 行)

**改进方案**:
```
gpui-entity/
├── SKILL.md (~80 行)
│   ├── Overview
│   ├── Quick Start
│   └── 引导文档
└── references/
    ├── entity-basics.md (创建和管理 entities)
    ├── lifecycle.md (Entity 生命周期)
    ├── updates.md (更新和通知模式)
    └── best-practices.md
```

#### 3. gpui-async (441 行 → 目标 <100 行)

**改进方案**:
```
gpui-async/
├── SKILL.md (~80 行)
└── references/
    ├── spawn-patterns.md (异步任务模式)
    ├── background-tasks.md (后台任务)
    ├── async-context.md (AsyncAppContext/AsyncWindowContext)
    └── best-practices.md
```

#### 4. gpui-context (429 行 → 目标 <100 行)

**改进方案**:
```
gpui-context/
├── SKILL.md (~80 行)
└── references/
    ├── app-context.md (App 上下文)
    ├── window-context.md (Window 上下文)
    ├── component-context.md (Context<T>)
    ├── async-context.md (异步上下文)
    └── best-practices.md
```

#### 5. gpui-event (414 行 → 目标 <100 行)

**改进方案**:
```
gpui-event/
├── SKILL.md (~80 行)
└── references/
    ├── event-types.md (事件类型)
    ├── subscriptions.md (订阅模式)
    ├── observers.md (观察者模式)
    └── best-practices.md
```

#### 6. gpui-focus-handle (393 行 → 目标 <100 行)

**改进方案**:
```
gpui-focus-handle/
├── SKILL.md (~80 行)
└── references/
    ├── focus-basics.md (焦点基础)
    ├── navigation.md (键盘导航)
    ├── focus-events.md (焦点事件)
    └── best-practices.md
```

### P1 优先级 - 尽快改进（1 个 skill）

#### 7. gpui-global (300 行 → 目标 <150 行)

**改进方案**:
```
gpui-global/
├── SKILL.md (~100 行)
│   ├── 保留核心概念和快速入门
│   └── 简化示例
└── references/
    ├── advanced-patterns.md (高级模式和详细示例)
    └── best-practices.md
```

### P2 优先级 - 可选改进（2 个 skills）

#### 8. gpui-layout-and-style (194 行)

**建议**: 当前可以接受，但可以考虑将最佳实践部分移到 references/best-practices.md

#### 9. gpui-action (187 行)

**建议**: 当前可以接受，但可以考虑将复杂示例移到 references/advanced-examples.md

### 无需改进 - 已符合标准（5 个 skills）

- ✅ gpui-test (94 行) - **最佳实践示例**
- ✅ github-pull-request-description (44 行)
- ✅ new-component (22 行)
- ✅ generate-component-documentation (20 行)
- ✅ generate-component-story (20 行)

## Description 改进建议

### 需要增强 description 的 skills

#### github-pull-request-description

**当前**:
```yaml
description: Write a description to description GitHub Pull Request.
```

**问题**: 语法错误（"description to description"），不够详细

**改进建议**:
```yaml
description: Write professional GitHub Pull Request descriptions with clear summaries and breaking changes documentation. Use when creating or updating PR descriptions, documenting API changes, or writing release notes for the gpui-component library.
```

#### new-component

**当前**:
```yaml
description: Create new GPUI components. Use when building components, writing UI elements, or creating new component implementations.
```

**改进建议** (更具体):
```yaml
description: Create new GPUI UI components following project patterns and conventions. Use when implementing new components in crates/ui/src, building stateless/stateful elements, or creating component APIs consistent with existing components like Button, Select, Input.
```

## 实施计划

### 阶段 1: P0 优先级（第 1-2 周）

1. **gpui-element** - 最大的 skill，优先处理
   - 分析现有内容并规划 references/ 结构
   - 创建 api-reference.md, layout-phase.md 等文件
   - 精简 SKILL.md 到核心内容

2. **gpui-entity, gpui-async, gpui-context**
   - 批量处理类似结构的 skills
   - 复用 gpui-element 的模板和模式

3. **gpui-event, gpui-focus-handle**
   - 完成所有 P0 skills

### 阶段 2: P1 优先级（第 3 周）

4. **gpui-global**
   - 轻度重构，移动高级内容到 references/

### 阶段 3: P2 和优化（第 4 周）

5. **gpui-layout-and-style, gpui-action**
   - 可选优化

6. **Description 改进**
   - 更新所有 skills 的 description

7. **质量检查**
   - 验证所有改进符合 skill-creator 标准
   - 确保一致性

## 通用改进模板

### SKILL.md 标准结构（目标 <100 行）

```markdown
---
name: skill-name
description: [详细的触发描述，包含使用场景]
---

## When to Use

[3-5 条明确的使用场景]

## Quick Start

[1-2 个最基础的示例，<30 行代码]

## Core Concepts

[关键概念简要说明，<50 行]

## Reference Documentation

[指向 references/ 中的详细文档]
- **[主题 1]**: See [file1.md](references/file1.md)
- **[主题 2]**: See [file2.md](references/file2.md)
- **Examples**: See examples/ directory
- **Best Practices**: See [best-practices.md](references/best-practices.md)
```

### references/ 目录组织

```
references/
├── api-reference.md      # 完整的 API 文档
├── [concept]-basics.md   # 基础概念详解
├── [concept]-advanced.md # 高级用法
├── patterns.md           # 常见模式
├── best-practices.md     # 最佳实践
└── troubleshooting.md    # 问题排查（如需要）
```

### examples/ 目录组织（如需要）

```
examples/
├── simple-[feature].md    # 简单示例
├── [feature]-patterns.md  # 特定功能的模式
└── complex-[feature].md   # 复杂示例
```

## 成功标准

改进后每个 skill 应该：

1. ✅ SKILL.md < 500 行（理想 < 150 行）
2. ✅ 使用渐进式披露（长内容拆分到 references/）
3. ✅ Description 详细说明触发条件
4. ✅ SKILL.md 包含核心概览和快速入门
5. ✅ 详细文档在 references/ 中，SKILL.md 明确引导
6. ✅ 结构清晰，易于导航
7. ✅ 遵循命令式语气（imperative/infinitive form）

## 参考资源

- Skill-creator 标准: `/Users/jason/.claude/skills/skill-creator/SKILL.md`
- 最佳实践示例: `.claude/skills/gpui-test/` (已正确使用渐进式披露)
- 工作流程模式: skill-creator 中的 `references/workflows.md`
- 输出模式: skill-creator 中的 `references/output-patterns.md`

## 预期收益

1. **性能提升**: 减少 70-90% 的上下文占用（如 gpui-element 从 757 行降到 ~80 行）
2. **可维护性**: 结构化的文档更易于更新和维护
3. **用户体验**: Claude 可以按需加载详细内容，提高响应速度
4. **一致性**: 所有 skills 遵循统一的结构和标准
5. **可扩展性**: 未来添加新内容时更容易组织

## 下一步行动

1. **获得批准**: 用户审查并批准此改进计划
2. **开始实施**: 按阶段顺序执行改进
3. **持续验证**: 每完成一个 skill 后进行测试
4. **文档化**: 记录改进过程中的最佳实践
