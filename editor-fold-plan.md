以下是一份“可直接进仓库”的设计文档草案，面向 **gpui-component**，以 **WrapMap + FoldMap** 为核心，把现有 `crates/ui/src/input/text_wrapper.rs` 的能力整合进来，并把 fold 做成显示投影层。整体目标：**Editor/Input 只依赖一个统一、易懂的显示映射 API**，同时避免引入任何 Zed GPL 风险（只借鉴概念，不借鉴实现表达）。

---

commit: 6903579a 为修改之前正确的版本，里面有很多核心关于 input 算法的细节例如 soft wrap，indent, ghost line 的处理等，后续 fold 的设计需要基于这个版本的 TextWrapper 来演进，才能保证软换行和命中测试的能力不丢失。以前的算法性能很高，能支持 20 万行编辑。

# WrapMap & FoldMap：显示映射管线设计文档

## 背景与动机

gpui-component 目前在输入系统中已有成熟的软换行/测量/命中测试组件：`crates/ui/src/input/text_wrapper.rs`（以下简称 **TextWrapper**）。它已经提供：

- 将文本拆分为逻辑行，并维护每行的软换行切片（`wrapped_lines`）
- `offset ↔ display_point` 的双向映射（这里的 display row 目前仅表示 soft-wrap 后的视觉行）
- 增量更新 `update(...)`，可在文本局部变化时只重算受影响行

目前缺少的能力是 **code fold（代码折叠）**。折叠的本质不是改动 buffer，而是改变“可见行集合”。若直接在 Editor 层做折叠，会导致：

- 光标/选择/命中测试/滚动高度计算到处充斥映射补丁
- 和 soft-wrap 的 row 语义互相污染，长期难维护

因此需要引入一个明确的显示映射管线：
**WrapMap（软换行映射）+ FoldMap（折叠投影）**，并提供对外统一 API，Editor/Input 不再关心内部细节。

---

## 目标（Goals）

1. **统一坐标系统**：对外提供稳定的 `BufferPos ↔ DisplayPos` 映射，其中 `DisplayPos.row` 表示最终可见行（已包含 wrap + fold）。
2. **复用现有 TextWrapper**：WrapMap 直接基于 TextWrapper 演进，避免重写软换行和命中测试。
3. **Fold 作为投影层**：FoldMap 不修改文本，只通过“可见 wrap_row 列表”投影出最终 display_row。
4. **易懂易用**：Editor 侧 API 要“看一眼就会用”，不暴露内部 wrap_row/缓存结构。
5. **可扩展**：未来可加入更多显示变换（inlay、block、tab 展开等），但本设计先把 wrap + fold 打通。

---

## 非目标（Non-goals）

- 不在本阶段实现复杂的折叠占位符（例如 `{ … }`、折叠行摘要），可作为后续迭代。
- 不在本阶段实现 Zed 那类 anchor/sum-tree 等高级增量结构；本设计优先可落地与可维护。
- 不在本阶段定义具体语言的 fold 规则全集；折叠候选范围由上层（tree-sitter/LSP）提供或逐步补齐。

---

## 名词与坐标系统

- **Buffer**：真实文本（Rope/字符串），按 `\n` 分割为逻辑行。
- **BufferPos**：`{ line, col }`，line 为逻辑行号，col 为列号（按字节或字符策略与现有一致）。
- **WrapRow**：软换行后的视觉行号（TextWrapper 当前的 display row 语义）。
- **DisplayRow**：最终展示行号（WrapRow 再经过 FoldMap 投影，隐藏折叠内部行）。
- **DisplayPos**：`{ row, col }`，row 为 DisplayRow，col 为视觉列。

约束：对外 **只暴露 BufferPos 和 DisplayPos**；WrapRow 属于内部实现细节。

---

## 总体架构

```
            ┌──────────────┐
Buffer/Rope │   Text Model  │
            └──────┬───────┘
                   │ text/layout change
                   ▼
            ┌──────────────┐
            │   WrapMap     │  (基于 TextWrapper)
            └──────┬───────┘
                   │ wrap_row space
                   ▼
            ┌──────────────┐
            │   FoldMap     │  (投影/过滤 wrap_row)
            └──────┬───────┘
                   │ display_row space
                   ▼
            ┌──────────────┐
            │  DisplayMap   │  (对外统一 API facade)
            └──────────────┘
```

> 说明：这里的 **DisplayMap** 是对外 facade（门面），内部仍以 WrapMap/FoldMap 分层实现。这样既清晰，也便于未来扩展更多 Map。

---

## 对外 API（Editor/Input 使用）

### 核心映射

```rust
struct BufferPos { line: usize, col: usize }
struct DisplayPos { row: usize, col: usize }

impl DisplayMap {
    fn display_row_count(&self) -> usize;

    fn buffer_pos_to_display_pos(&self, pos: BufferPos) -> DisplayPos;
    fn display_pos_to_buffer_pos(&self, pos: DisplayPos) -> BufferPos;

    fn display_row_to_buffer_line(&self, display_row: usize) -> usize;
    fn buffer_line_to_first_display_row(&self, buffer_line: usize) -> Option<usize>;
}
```

### 折叠控制

```rust
impl DisplayMap {
    fn set_folded(&mut self, start_line: usize, folded: bool);
    fn toggle_fold_at_line(&mut self, start_line: usize);
    fn clear_folds(&mut self);

    // 可选：用于 UI 渲染折叠图标/候选范围
    fn fold_candidates(&self) -> &[FoldRange];
    fn is_fold_start(&self, line: usize) -> bool;
    fn is_folded_start(&self, line: usize) -> bool;
}
```

### 更新入口

```rust
impl DisplayMap {
    fn on_text_changed(&mut self, change: TextChange);
    fn on_layout_changed(&mut self, wrap_width: Pixels, font: FontParams);
    fn on_fold_candidates_changed(&mut self, candidates: Vec<FoldRange>);
}
```

---

## 内部数据结构

### WrapMap（基于 TextWrapper）

WrapMap 直接复用 TextWrapper 的核心结构与算法，职责：

- 将 buffer 逻辑行拆分并做软换行切片
- 提供：
  - `buffer_pos ↔ wrap_pos`
  - wrap_row 的总数 `wrap_row_count()`
  - 命中测试所需的行布局/测量能力（延续现有 `LineLayout`）

内部接口（不对外暴露）示意：

```rust
struct WrapPos { row: usize, col: usize } // row=wrap_row

impl WrapMap {
    fn wrap_row_count(&self) -> usize;

    fn buffer_pos_to_wrap_pos(&self, pos: BufferPos) -> WrapPos;
    fn wrap_pos_to_buffer_pos(&self, pos: WrapPos) -> BufferPos;

    fn wrap_row_to_buffer_line(&self, wrap_row: usize) -> usize;
    fn buffer_line_to_first_wrap_row(&self, line: usize) -> usize;

    fn on_text_changed(&mut self, change: TextChange);
    fn on_layout_changed(&mut self, wrap_width: Pixels, font: FontParams);
}
```

> 备注：`wrap_row_to_buffer_line` 与 `buffer_line_to_first_wrap_row` 可通过 TextWrapper 的 `LineItem.lines_len()`（每个逻辑行占多少 wrap_row）累积得到前缀和数组/缓存，避免每次查询 O(n)。

### FoldRange 与折叠状态

```rust
struct FoldRange {
    start_line: usize, // buffer line
    end_line: usize,   // inclusive
}
```

FoldMap 内部维护两类集合：

1. `candidates: Vec<FoldRange>`：可折叠候选（来自 tree-sitter/LSP）
2. `folded: Vec<FoldRange>`：已折叠集合（通常是 candidates 的子集，按 start_line 唯一）

### FoldMap（wrap_row 投影）

FoldMap 核心是两个映射表（简单、可调试）：

- `visible_wrap_rows: Vec<usize>`
  - index = display_row
  - value = wrap_row（该 display 行实际对应的 wrap_row）

- `wrap_row_to_display_row: Vec<Option<usize>>`
  - index = wrap_row
  - value = Some(display_row) 或 None（被折叠隐藏）

FoldMap 同时需要 WrapMap 提供的 **buffer_line ↔ wrap_row 边界信息**（例如 `buffer_line_to_first_wrap_row` + 每行 wrap_len 前缀和）。

---

## 关键算法

### 1) FoldMap 重建映射（最关键）

当以下任一发生时需要重建 FoldMap：

- wrap 结构变化（文本变更或 wrap_width/font 变化）导致 wrap_row_count 变化
- folded 状态变化
- candidates 变化（可能影响 toggle 行的匹配范围）

重建过程：

1. 由 WrapMap 生成 `buffer_line_start_wrap_row[line]` 与 `buffer_line_end_wrap_row[line]`
2. 将 `folded` 变换为要隐藏的 wrap_row 区间集合：
   - 通常隐藏：从 `start_line + 1` 的起始 wrap_row，到 `end_line` 的末尾 wrap_row

3. 线性扫描 `wrap_row = 0..wrap_row_count`：
   - 若 wrap_row 在任何隐藏区间内 → 跳过（不可见）
   - 否则 push 到 `visible_wrap_rows` 并记录反向映射

复杂度：O(W + F)（W=wrap_row_count，F=folded 区间数）。
对多数编辑场景足够。

> 后续优化：当文件超大且频繁 fold/unfold，可用区间合并/二分查找加速“wrap_row 是否被隐藏”的判定；必要时再上 Fenwick tree，但不在本阶段。

### 2) `BufferPos -> DisplayPos`

流程：

1. WrapMap：`buffer_pos_to_wrap_pos` 得到 `(wrap_row, wrap_col)`
2. FoldMap：`wrap_row_to_display_row[wrap_row]`：
   - 若 Some(dr)：返回 `{ row: dr, col: wrap_col }`
   - 若 None（光标落在折叠内部）：
     - 策略：clamp 到折叠起点行的最后可见 wrap_row 对应的 display_row（或起点行 display_row）
     - 该策略需要在 FoldMap 内提供辅助：`nearest_visible_display_row_for_wrap_row(wrap_row)`

### 3) `DisplayPos -> BufferPos`

流程：

1. FoldMap：`visible_wrap_rows[display_row] -> wrap_row`
2. WrapMap：`wrap_pos_to_buffer_pos({wrap_row, col}) -> buffer_pos`

### 4) 渲染循环

Editor 渲染不再直接遍历 wrap_row，而是：

- 根据 scroll_y 算可见 display_row 区间
- display_row → wrap_row（用 `visible_wrap_rows`）
- wrap_row 的绘制继续复用现有 TextWrapper/LineLayout 逻辑

### 5) 命中测试（鼠标）

- y → display_row
- display_row → wrap_row
- wrap_row + x → offset/col（复用 WrapMap 现有布局命中）
- offset → BufferPos（WrapMap）

---

## Fold 候选范围来源（tree-sitter/LSP）

DisplayMap 不负责语言解析，仅接受输入：

- `on_fold_candidates_changed(candidates: Vec<FoldRange>)`

上层可能来源：

- tree-sitter 遍历语法树节点（按 kind 白名单）
- LSP `foldingRange` 结果
- 其他语言服务

DisplayMap 的约束：

- candidates 按 start_line 排序
- start_line 唯一化（同起点多范围择优：优先更大范围或最稳定规则）

toggle 行为：

- `toggle_fold_at_line(line)`：从 candidates 中找到 `start_line == line` 的范围，加入/移出 folded

---

## 文本编辑下的 folded 稳定性策略

短期保守策略（建议先这样落地）：

- 文本变更后，如果变更涉及 folded 区域或行号大幅变化：
  - 清空 folded（保证正确性，牺牲体验）

- 否则尝试按 start_line retain（当插入/删除行发生在折叠区之前，会漂移；简单 retain 会错位）

中期改进（后续迭代）：

- 记录折叠范围的额外锚点信息（例如起点附近文本 hash、语法节点 byte range）
- candidates 更新后，用锚点重新定位折叠起点

---

## 滚动与布局

折叠改变 `display_row_count()`，需要：

- 折叠/展开后 clamp scroll_y，避免滚动超过最大值
- 计算内容高度以 display_row_count 为准：
  - `content_height = display_row_count * line_height`（或基于可变行高策略扩展）

---

## 测试计划

1. **映射一致性测试（核心）**

- 对随机 BufferPos：
  - `buffer -> display -> buffer` 应该回到同一位置（允许折叠 clamp 情况下回到等价可见位置）

- 对随机 DisplayPos：
  - `display -> buffer -> display` 回到同一 display 行

2. **折叠投影测试**

- 给定 candidates + folded，验证：
  - display_row_count 正确
  - 被折叠范围内 wrap_row 均不可见（wrap_row_to_display_row 为 None）

3. **增量更新回归**

- 文本插入/删除后 WrapMap 增量更新正确
- FoldMap 在更新后重建，映射不 panic、不越界

4. **UI 交互测试（可手动）**

- gutter 点击折叠图标：折叠/展开正确
- 上下键移动光标：不会进入隐藏行
- 鼠标点击：命中与光标位置一致

---

## 迁移与落地步骤（推荐拆 PR）

1. PR1：引入 WrapMap（封装 TextWrapper），保持现有行为不变
2. PR2：引入 FoldMap + DisplayMap facade，先提供手工折叠 API（无 candidates）
3. PR3：Editor/Input 改用 DisplayMap 的渲染与命中 API
4. PR4：接入 candidates（tree-sitter/LSP），实现 gutter 折叠 UI
5. PR5：折叠稳定性与滚动 clamp、缓存优化

---

## 风险与缓解

- **风险：row 语义混淆（wrap_row vs display_row）**
  缓解：对外只暴露 DisplayPos；内部类型命名严格区分 WrapPos/DisplayPos。

- **风险：折叠后光标落在隐藏范围**
  缓解：提供 clamp 策略，统一在 DisplayMap 内处理，Editor 不做特殊判断。

- **风险：大文件性能**
  缓解：先线性重建映射；必要时优化隐藏区间判定或引入 Fenwick tree（经典算法，不依赖外部实现）。

- **合规风险：避免 GPL 传染**
  缓解：
  - 只采用“显示为投影/分层映射”的通用架构思想
  - 不参考/不复用任何 GPL 代码表达、类型布局、函数签名
  - WrapMap 完全基于本项目现有 TextWrapper 演进，FoldMap 为独立实现

---

## 附：为什么这个设计足够“简单易懂”

- Editor 侧只需要记住：**DisplayMap 是唯一真理**
- `DisplayRow` 就是屏幕上的行；fold 隐藏的行根本不存在于 DisplayRow 空间
- wrap/fold 的复杂映射都被收敛到 DisplayMap 内部，避免“全项目到处写 if folded then …”

---

如果你愿意，我可以基于你们当前 `TextWrapper` 的具体函数（比如 `offset_to_display_point` / `display_point_to_offset` / `lines`、`lines_len()` 等）把 **WrapMap 的内部接口**和 **FoldMap 重建映射的伪代码**写得更贴近你们仓库现状，做到你直接照着拆文件就能开工。
