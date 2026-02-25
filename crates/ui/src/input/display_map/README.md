# Display Mapping System

显示映射系统为 Editor/Input 提供统一的坐标转换和代码折叠功能。

## 架构设计

基于分层投影模式：

```
Buffer (Rope)              逻辑文本
    ↓
WrapMap                    软换行层
    ↓ (wrap_row)
FoldMap                    折叠投影层
    ↓ (display_row)
DisplayMap (Facade)        统一的公共 API
```

### 坐标系统

1. **BufferPos** `{ line, col }` - 缓冲区逻辑坐标
   - line: 逻辑行号 (按 \n 分割)
   - col: 列号（字节偏移）

2. **WrapPos** `{ row, col }` - 软换行后的坐标（内部使用）
   - row: wrap_row（软换行后的视觉行）
   - col: 视觉列

3. **DisplayPos** `{ row, col }` - 最终显示坐标（对外 API）
   - row: display_row（折叠后的可见行）
   - col: 显示列

## 模块职责

### DisplayMap (`display_map.rs`)

**对外统一接口（Facade）**

主要功能：
- BufferPos ↔ DisplayPos 转换
- 折叠控制 (set_folded, toggle_fold, clear_folds)
- 行数查询 (display_row_count, buffer_line_count)
- 文本更新 (on_text_changed, on_layout_changed)

临时提供（用于渐进迁移）：
- `wrap_map()` - 访问 WrapMap 层
- `fold_map()` - 访问 FoldMap 层

### WrapMap (`wrap_map.rs`)

**软换行映射层**

基于 TextWrapper 实现，提供：
- Buffer ↔ Wrap 坐标转换
- wrap_row ↔ buffer_line 查询
- 前缀和缓存优化查询性能

核心方法：
- `buffer_pos_to_wrap_pos(pos)` - Buffer → Wrap
- `wrap_pos_to_buffer_pos(pos)` - Wrap → Buffer
- `buffer_line_to_first_wrap_row(line)` - 行号 → 首个 wrap_row
- `wrap_row_to_buffer_line(row)` - wrap_row → 行号

### FoldMap (`fold_map.rs`)

**折叠投影层**

通过过滤 wrap_row 实现折叠：
- 维护可见 wrap_row 列表
- Wrap ↔ Display 双向映射
- 处理折叠状态变化

数据结构：
- `visible_wrap_rows: Vec<usize>` - display_row → wrap_row
- `wrap_row_to_display_row: Vec<Option<usize>>` - wrap_row → display_row
- `candidates: Vec<FoldRange>` - 折叠候选
- `folded: Vec<FoldRange>` - 已折叠范围

### Types (`types.rs`)

定义核心坐标类型：
- `BufferPos` - 缓冲区位置（公开）
- `WrapPos` - 软换行位置（内部）
- `DisplayPos` - 显示位置（公开）
- `FoldRange` - 折叠范围（公开）

## 使用示例

### 基本坐标转换

```rust
use crate::input::{DisplayMap, BufferPos, DisplayPos};

let mut display_map = DisplayMap::new(font, font_size, wrap_width);
display_map.set_text(&rope, cx);

// Buffer → Display
let buffer_pos = BufferPos::new(10, 5);
let display_pos = display_map.buffer_pos_to_display_pos(buffer_pos);

// Display → Buffer
let buffer_pos = display_map.display_pos_to_buffer_pos(display_pos);
```

### 代码折叠

```rust
use crate::input::FoldRange;

// 设置折叠候选（来自 tree-sitter/LSP）
let candidates = vec![
    FoldRange::new(10, 15),  // 折叠 10-15 行
    FoldRange::new(20, 25),  // 折叠 20-25 行
];
display_map.set_fold_candidates(candidates);

// 切换折叠状态
display_map.toggle_fold(10);  // 折叠/展开第 10 行

// 查询折叠状态
if display_map.is_folded_at(10) {
    println!("第 10 行已折叠");
}

// 清除所有折叠
display_map.clear_folds();
```

### 文本更新

```rust
// 文本变化
display_map.on_text_changed(&changed_text, &range, &new_text, cx);

// 布局变化（wrap width 改变）
display_map.on_layout_changed(Some(new_width), cx);

// 字体变化
display_map.set_font(font, font_size, cx);
```

### 访问底层（渐进迁移期间）

```rust
// 访问 TextWrapper (用于现有渲染代码)
let wrapper = display_map.wrap_map().wrapper();
let lines = wrapper.lines;
let longest_row = wrapper.longest_row;

// 访问 WrapMap
let wrap_count = display_map.wrap_map().wrap_row_count();

// 访问 FoldMap
let folded_ranges = display_map.fold_map().folded_ranges();
```

## 性能特性

### O(1) 操作
- `display_row_count()` - 预计算缓存
- `buffer_line_to_first_wrap_row()` - 前缀和数组
- `wrap_row_to_display_row()` - 直接数组查询

### O(log n) 操作
- `wrap_row_to_buffer_line()` - 二分搜索

### 增量更新
- 文本变化：只重算受影响的行（由 TextWrapper 提供）
- 折叠变化：重建 FoldMap 映射（通常很快）

## 设计原则

1. **分离关注点**
   - WrapMap：只关心软换行
   - FoldMap：只关心折叠
   - DisplayMap：统一对外接口

2. **单向依赖**
   ```
   FoldMap → WrapMap → TextWrapper
   (上层依赖下层，下层不知道上层)
   ```

3. **内部细节隐藏**
   - WrapPos 不对外暴露
   - 外部只需要知道 BufferPos 和 DisplayPos

4. **渐进迁移支持**
   - 提供 wrap_map()/fold_map() 访问器
   - 允许现有代码平滑过渡

## 未来扩展

本架构预留了扩展空间：

- **Inlay Hints** - 可作为新的映射层
- **Block Decorations** - 插入虚拟行
- **Tab Expansion** - 制表符展开
- **Diff Mapping** - diff 视图的行映射

扩展方式：在 WrapMap 和 FoldMap 之间插入新层，DisplayMap 保持不变。

## 参考文档

- [设计文档](../../../../editor-fold-plan.md) - 完整的架构设计
- [迁移指南](./MIGRATION.md) - 从 TextWrapper 迁移到 DisplayMap
