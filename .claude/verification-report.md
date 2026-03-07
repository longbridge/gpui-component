# 验证报告

- 任务：为 MSSQL 视图插件补充“导出脚本”菜单
- 时间：2026-03-07 22:15:10 +0800
- 结论：通过
- 综合评分：95/100

## 技术维度评分
- 代码质量：96/100
  - 改动集中在 `crates/db_view/src/mssql/mssql_view_plugin.rs`，只补菜单接线，不扩散逻辑。
  - 直接复用 `DbTreeViewEvent::DumpSqlFile` 与 `SqlDumpMode`，没有新增并行实现。
- 测试覆盖：88/100
  - 已通过 `cargo fmt --all` 与 `cargo check -p db_view`。
  - 当前未补自动化 UI 菜单测试，保留少量扣分。
- 规范遵循：100/100
  - 命名、菜单组织方式、改动范围均与其他数据库插件一致。

## 战略维度评分
- 需求匹配：97/100
  - 已让 MSSQL 在 Database / Table 节点具备与其他数据库一致的导出脚本入口。
- 架构一致：95/100
  - 复用现有导出事件、导出视图与后端 SQL 导出链路，没有破坏模块边界。
- 风险评估：93/100
  - 菜单层改动风险低。
  - 剩余风险主要在于 MSSQL 通用 SQL 导出内容是否满足更复杂对象定义，这属于后端导出质量问题，不是本次接线问题。

## 本地验证
- `cargo fmt --all`
- `cargo check -p db_view`

## 变更摘要
- `crates/db_view/src/mssql/mssql_view_plugin.rs`
  - 引入 `SqlDumpMode`
  - 在 `DbNodeType::Database` 菜单中新增“导出结构 / 导出数据 / 导出结构和数据”子菜单
  - 在 `DbNodeType::Table` 菜单中新增对应的导出脚本子菜单

## 建议
- 当前实现可以直接使用。
- 若后续想减少各数据库插件重复维护，可把 dump_sql 子菜单抽成共享 helper。
- 若要进一步验证导出内容正确性，可后续补一次 MSSQL 实机导出冒烟验证。