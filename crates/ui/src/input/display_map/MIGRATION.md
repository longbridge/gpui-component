# DisplayMap 迁移指南

本文档说明如何从直接使用 `TextWrapper` 迁移到使用 `DisplayMap`。

## 架构概述

```
DisplayMap (公共 facade)
  ├─ wrap_map: WrapMap (软换行层)
  │    └─ wrapper: TextWrapper (内部实现)
  └─ fold_map: FoldMap (折叠层)
```

## 当前阶段：渐进迁移

### 方式 1：通过 DisplayMap 访问底层（推荐用于现有代码）

```rust
// 获取 TextWrapper
let text_wrapper = display_map.wrap_map().wrapper();

// 获取 lines
let lines = display_map.wrap_map().lines();

// 获取文本
let text = display_map.text();
```

### 方式 2：使用 DisplayMap API（推荐用于新代码）

```rust
// 坐标转换
let buffer_pos = BufferPos::new(10, 5);
let display_pos = display_map.buffer_pos_to_display_pos(buffer_pos);

// 行数查询
let total_rows = display_map.display_row_count();
let buffer_lines = display_map.buffer_line_count();

// 折叠控制
display_map.set_folded(start_line, true);
display_map.toggle_fold(line);
```

## API 对照表

### 坐标转换

| 旧方式 (TextWrapper) | 新方式 (DisplayMap) |
|---------------------|-------------------|
| `text_wrapper.offset_to_display_point(offset)` | `display_map.wrap_map().wrapper().offset_to_display_point(offset)` |
| `text_wrapper.display_point_to_offset(point)` | `display_map.wrap_map().wrapper().display_point_to_offset(point)` |
| N/A | `display_map.buffer_pos_to_display_pos(pos)` ✨ |
| N/A | `display_map.display_pos_to_buffer_pos(pos)` ✨ |

### 行数查询

| 旧方式 | 新方式 |
|-------|-------|
| `text_wrapper.len()` | `display_map.wrap_row_count()` |
| `text_wrapper.lines.len()` | `display_map.buffer_line_count()` |
| N/A | `display_map.display_row_count()` ✨ (折叠后的可见行数) |

### 文本访问

| 旧方式 | 新方式 |
|-------|-------|
| `&text_wrapper.text` | `display_map.text()` |
| `&text_wrapper.lines` | `display_map.wrap_map().lines()` |
| `text_wrapper.longest_row` | `display_map.wrap_map().wrapper().longest_row` |

### 更新操作

| 旧方式 | 新方式 |
|-------|-------|
| `text_wrapper.update(...)` | `display_map.on_text_changed(...)` |
| `text_wrapper.set_wrap_width(...)` | `display_map.on_layout_changed(...)` |
| `text_wrapper.set_font(...)` | `display_map.set_font(...)` |
| `text_wrapper.set_default_text(...)` | `display_map.set_text(...)` |

## 迁移步骤

### 第一阶段：替换字段（当前）

```rust
// 旧代码
pub struct InputState {
    text_wrapper: TextWrapper,
}

// 新代码
pub struct InputState {
    display_map: DisplayMap,
}
```

### 第二阶段：更新调用点

```rust
// 旧代码
self.text_wrapper.len()

// 过渡代码（功能不变）
self.display_map.wrap_map().wrapper().len()

// 目标代码（使用新 API）
self.display_map.wrap_row_count()
```

### 第三阶段：利用新功能

```rust
// 使用代码折叠
display_map.set_fold_candidates(candidates);
display_map.toggle_fold(line);

// 使用统一的坐标转换
let display_pos = display_map.buffer_pos_to_display_pos(buffer_pos);
```

## 注意事项

1. **WrapRow vs DisplayRow**
   - WrapRow: 软换行后的行（内部概念）
   - DisplayRow: 折叠后的最终可见行（对外 API）
   - 通常应该使用 DisplayRow (通过 DisplayPos)

2. **向后兼容**
   - `wrap_map()` 和 `fold_map()` 方法是临时的迁移辅助
   - 未来可能会移除，建议逐步迁移到纯 DisplayMap API

3. **性能**
   - DisplayMap 内部已优化，无需担心多层调用的性能开销

## 示例：完整迁移

### 迁移前
```rust
impl InputState {
    fn move_cursor(&mut self, lines: isize) {
        let offset = self.selection.head;
        let display_point = self.text_wrapper.offset_to_display_point(offset);
        let new_row = (display_point.row as isize + lines).max(0);
        let new_point = DisplayPoint::new(new_row as usize, 0, display_point.column);
        let new_offset = self.text_wrapper.display_point_to_offset(new_point);
        self.selection.head = new_offset;
    }
}
```

### 迁移后（过渡）
```rust
impl InputState {
    fn move_cursor(&mut self, lines: isize) {
        let offset = self.selection.head;
        let wrapper = self.display_map.wrap_map().wrapper();
        let display_point = wrapper.offset_to_display_point(offset);
        let new_row = (display_point.row as isize + lines).max(0);
        let new_point = DisplayPoint::new(new_row as usize, 0, display_point.column);
        let new_offset = wrapper.display_point_to_offset(new_point);
        self.selection.head = new_offset;
    }
}
```

### 迁移后（理想）
```rust
impl InputState {
    fn move_cursor(&mut self, lines: isize) {
        // TODO: 使用 DisplayMap 的 BufferPos ↔ DisplayPos API
        // 这需要重构 offset-based 的坐标系统
    }
}
```
