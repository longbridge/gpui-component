use std::ops::Range;

/// 单元格坐标 (row, col)
pub type CellCoord = (usize, usize);

/// 单元格范围（矩形选区）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CellRange {
    /// 选择起点
    pub start: CellCoord,
    /// 选择终点
    pub end: CellCoord,
}

impl CellRange {
    /// 创建新的单元格范围
    pub fn new(start: CellCoord, end: CellCoord) -> Self {
        Self { start, end }
    }

    /// 创建单个单元格的范围
    pub fn single(coord: CellCoord) -> Self {
        Self {
            start: coord,
            end: coord,
        }
    }

    /// 返回标准化的范围 (min, max)
    pub fn normalized(&self) -> (CellCoord, CellCoord) {
        let min_row = self.start.0.min(self.end.0);
        let max_row = self.start.0.max(self.end.0);
        let min_col = self.start.1.min(self.end.1);
        let max_col = self.start.1.max(self.end.1);
        ((min_row, min_col), (max_row, max_col))
    }

    /// 检查指定单元格是否在范围内
    pub fn contains(&self, row: usize, col: usize) -> bool {
        let ((min_row, min_col), (max_row, max_col)) = self.normalized();
        row >= min_row && row <= max_row && col >= min_col && col <= max_col
    }

    /// 获取行范围
    pub fn row_range(&self) -> Range<usize> {
        let ((min_row, _), (max_row, _)) = self.normalized();
        min_row..(max_row + 1)
    }

    /// 获取列范围
    pub fn col_range(&self) -> Range<usize> {
        let ((_, min_col), (_, max_col)) = self.normalized();
        min_col..(max_col + 1)
    }

    /// 获取范围内的所有单元格坐标
    pub fn cells(&self) -> Vec<CellCoord> {
        let mut result = Vec::new();
        let ((min_row, min_col), (max_row, max_col)) = self.normalized();
        for row in min_row..=max_row {
            for col in min_col..=max_col {
                result.push((row, col));
            }
        }
        result
    }

    /// 获取范围内的单元格数量
    pub fn cell_count(&self) -> usize {
        let ((min_row, min_col), (max_row, max_col)) = self.normalized();
        (max_row - min_row + 1) * (max_col - min_col + 1)
    }

    /// 检查是否为单个单元格
    pub fn is_single(&self) -> bool {
        self.start == self.end
    }

    /// 扩展范围到指定单元格
    pub fn extend_to(&mut self, coord: CellCoord) {
        self.end = coord;
    }
}

impl Default for CellRange {
    fn default() -> Self {
        Self {
            start: (0, 0),
            end: (0, 0),
        }
    }
}

/// 表格选区状态
#[derive(Debug, Clone, Default)]
pub struct TableSelection {
    /// 选中的范围（支持多选）
    pub ranges: Vec<CellRange>,
    /// 选择锚点（起点）
    pub anchor: Option<CellCoord>,
    /// 当前活动单元格（光标位置）
    pub active: Option<CellCoord>,
}

impl TableSelection {
    /// 创建新的空选区
    pub fn new() -> Self {
        Self::default()
    }

    /// 检查选区是否为空
    pub fn is_empty(&self) -> bool {
        self.ranges.is_empty() && self.active.is_none()
    }

    /// 清除所有选区
    pub fn clear(&mut self) {
        self.ranges.clear();
        self.anchor = None;
        self.active = None;
    }

    /// 选择单个单元格（替换现有选区）
    pub fn select_single(&mut self, coord: CellCoord) {
        self.ranges.clear();
        self.ranges.push(CellRange::single(coord));
        self.anchor = Some(coord);
        self.active = Some(coord);
    }

    /// 扩展选区到指定单元格（Shift+Click）
    pub fn extend_to(&mut self, coord: CellCoord) {
        if let Some(anchor) = self.anchor {
            self.ranges.clear();
            self.ranges.push(CellRange::new(anchor, coord));
            self.active = Some(coord);
        } else {
            self.select_single(coord);
        }
    }

    /// 添加单元格到选区（Ctrl+Click）
    pub fn add(&mut self, coord: CellCoord) {
        // 检查是否已经选中，如果是则取消选中
        if self.contains(coord.0, coord.1) {
            self.remove(coord);
        } else {
            self.ranges.push(CellRange::single(coord));
            self.anchor = Some(coord);
            self.active = Some(coord);
        }
    }

    /// 从选区中移除单元格
    pub fn remove(&mut self, coord: CellCoord) {
        self.ranges.retain(|range| {
            if range.is_single() && range.start == coord {
                false
            } else if range.contains(coord.0, coord.1) {
                // 对于范围选择，这里简化处理：如果点击的是范围内的单元格，不做移除
                // 完整实现需要拆分范围，这里暂时保留
                true
            } else {
                true
            }
        });

        // 更新 active
        if self.active == Some(coord) {
            self.active = self.ranges.last().map(|r| r.end);
        }
    }

    /// 检查指定单元格是否在选区内
    pub fn contains(&self, row: usize, col: usize) -> bool {
        self.ranges.iter().any(|range| range.contains(row, col))
    }

    /// 获取所有选中的单元格坐标
    pub fn all_cells(&self) -> Vec<CellCoord> {
        let mut result = Vec::new();
        for range in &self.ranges {
            result.extend(range.cells());
        }
        // 去重
        result.sort();
        result.dedup();
        result
    }

    /// 获取第一个选中范围（用于复制）
    pub fn first_range(&self) -> Option<&CellRange> {
        self.ranges.first()
    }

    /// 获取选中的单元格数量
    pub fn cell_count(&self) -> usize {
        self.all_cells().len()
    }

    /// 更新拖选时的当前范围
    pub fn update_drag(&mut self, coord: CellCoord) {
        if let Some(anchor) = self.anchor {
            if let Some(last_range) = self.ranges.last_mut() {
                last_range.end = coord;
            } else {
                self.ranges.push(CellRange::new(anchor, coord));
            }
            self.active = Some(coord);
        }
    }

    /// 开始新的拖选
    pub fn start_drag(&mut self, coord: CellCoord, add_to_selection: bool) {
        if !add_to_selection {
            self.ranges.clear();
        }
        self.anchor = Some(coord);
        self.active = Some(coord);
        self.ranges.push(CellRange::single(coord));
    }

    /// 选择行
    pub fn select_row(&mut self, row: usize, start_col: usize, end_col: usize) {
        self.ranges.clear();
        self.ranges
            .push(CellRange::new((row, start_col), (row, end_col)));
        self.anchor = Some((row, start_col));
        self.active = Some((row, start_col));
    }

    /// 选择列
    pub fn select_col(&mut self, col: usize, row_count: usize) {
        self.ranges.clear();
        self.ranges
            .push(CellRange::new((0, col), (row_count.saturating_sub(1), col)));
        self.anchor = Some((0, col));
        self.active = Some((0, col));
    }

    /// 选择所有单元格
    pub fn select_all(&mut self, row_count: usize, col_count: usize) {
        self.ranges.clear();
        if row_count > 0 && col_count > 0 {
            self.ranges.push(CellRange::new(
                (0, 0),
                (row_count.saturating_sub(1), col_count.saturating_sub(1)),
            ));
            self.anchor = Some((0, 0));
            self.active = Some((0, 0));
        }
    }

    /// 移动活动单元格
    pub fn move_active(
        &mut self,
        row_delta: i32,
        col_delta: i32,
        row_count: usize,
        col_count: usize,
    ) {
        if let Some((row, col)) = self.active {
            let new_row =
                (row as i32 + row_delta).clamp(0, row_count.saturating_sub(1) as i32) as usize;
            let new_col =
                (col as i32 + col_delta).clamp(0, col_count.saturating_sub(1) as i32) as usize;
            self.select_single((new_row, new_col));
        } else if row_count > 0 && col_count > 0 {
            self.select_single((0, 0));
        }
    }

    /// 扩展选区（Shift+方向键）
    pub fn extend_active(
        &mut self,
        row_delta: i32,
        col_delta: i32,
        row_count: usize,
        col_count: usize,
    ) {
        if let Some((row, col)) = self.active {
            let new_row =
                (row as i32 + row_delta).clamp(0, row_count.saturating_sub(1) as i32) as usize;
            let new_col =
                (col as i32 + col_delta).clamp(0, col_count.saturating_sub(1) as i32) as usize;
            self.extend_to((new_row, new_col));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cell_range_contains() {
        let range = CellRange::new((1, 1), (3, 3));
        assert!(range.contains(1, 1));
        assert!(range.contains(2, 2));
        assert!(range.contains(3, 3));
        assert!(!range.contains(0, 0));
        assert!(!range.contains(4, 4));
    }

    #[test]
    fn test_cell_range_normalized() {
        let range = CellRange::new((3, 3), (1, 1));
        let (min, max) = range.normalized();
        assert_eq!(min, (1, 1));
        assert_eq!(max, (3, 3));
    }

    #[test]
    fn test_selection_single() {
        let mut selection = TableSelection::new();
        selection.select_single((1, 2));
        assert!(selection.contains(1, 2));
        assert!(!selection.contains(0, 0));
        assert_eq!(selection.active, Some((1, 2)));
    }

    #[test]
    fn test_selection_extend() {
        let mut selection = TableSelection::new();
        selection.select_single((1, 1));
        selection.extend_to((3, 3));
        assert!(selection.contains(1, 1));
        assert!(selection.contains(2, 2));
        assert!(selection.contains(3, 3));
        assert!(!selection.contains(0, 0));
    }
}
