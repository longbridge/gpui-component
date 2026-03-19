## 项目上下文摘要（csv-import-fix）
生成时间：2026-03-19 14:33:08 +0800

### 1. 相似实现分析
- 实现1：`crates/db/src/import_export/formats/csv.rs`
  - 模式：`FormatHandler` 下的导入/导出双实现
  - 可复用：CSV 解析与 SQL INSERT 拼接主流程
  - 风险点：`Option<String>` 值转换错误导致编译失败
- 实现2：`crates/db/src/import_export/formats/json.rs`
  - 模式：逐行对象转 INSERT，统一错误收集
  - 可复用：值到 SQL 字面量映射策略
- 实现3：`crates/db/src/import_export/formats/txt.rs`
  - 模式：与 CSV 类似的按行解析 + INSERT
  - 可复用：列数校验、`stop_on_error` 分支处理

### 2. 项目约定
- 命名约定：Rust `snake_case`，类型 `PascalCase`
- 文件组织：按 `import_export/formats/*` 分格式拆分
- 错误处理：`anyhow` + `Result`，保留错误列表到 `ImportResult.errors`

### 3. 可复用组件清单
- `crates/db/src/import_export/formats/json.rs`：JSON 值映射与 INSERT 模式
- `crates/db/src/import_export/formats/txt.rs`：文本导入列校验与 SQL 执行模式
- `crates/db/src/plugin.rs`：按 `DataFormat` 分发到对应 `FormatHandler`

### 4. 测试策略
- 测试框架：Rust 内置 `#[test]`
- 参考模式：`crates/db/src/mssql/plugin.rs` 的 `#[cfg(test)] mod tests`
- 本次新增：在 `csv.rs` 内补充单元测试，覆盖空字符串/NULL/引号转义

### 5. 依赖与集成点
- 入口：`db_view` 组装 `ImportConfig` 后调用 `import_data_with_progress_sync`
- 后端：`plugin.rs` 按 `DataFormat::Csv` 调用 `CsvFormatHandler`
- 影响范围：`crates/db` 内部实现，无接口签名变化

### 6. 技术选型理由
- 选择最小改动修复：只修正 `Option<String>` 转 SQL 的错误分支
- 优势：风险低、可快速恢复构建与导入行为
- 后续可选优化：迁移到 `csv::ReaderBuilder` 统一处理复杂 CSV 场景
