//! DDL 失效器模块
//!
//! 负责解析 SQL 语句，检测 DDL 操作，并触发相应的缓存失效。
//! 优先使用 sqlparser AST 解析，失败时 fallback 到字符串匹配。

use std::sync::Arc;
use tracing::{debug, warn};

use sqlparser::ast::Statement;
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;

use crate::metadata_cache::{CacheKey, MetadataCacheManager};

/// DDL 关键字列表，用于粗粒度兜底检测
const DDL_KEYWORDS: &[&str] = &["CREATE", "ALTER", "DROP", "TRUNCATE", "RENAME"];

/// DDL 事件类型
#[derive(Debug, Clone)]
pub enum DdlEvent {
    /// 创建表
    CreateTable {
        database: String,
        schema: Option<String>,
        table: String,
    },
    /// 修改表
    AlterTable {
        database: String,
        schema: Option<String>,
        table: String,
    },
    /// 删除表
    DropTable {
        database: String,
        schema: Option<String>,
        table: String,
    },
    /// 创建数据库
    CreateDatabase { database: String },
    /// 删除数据库
    DropDatabase { database: String },
    /// 清空表
    TruncateTable {
        database: String,
        schema: Option<String>,
        table: String,
    },
    /// 重命名表
    RenameTable {
        database: String,
        schema: Option<String>,
        old_table: String,
        new_table: String,
    },
    /// 创建索引
    CreateIndex {
        database: String,
        schema: Option<String>,
        table: String,
        index: String,
    },
    /// 删除索引
    DropIndex {
        database: String,
        schema: Option<String>,
        table: String,
        index: String,
    },
    /// 创建视图
    CreateView {
        database: String,
        schema: Option<String>,
        view: String,
    },
    /// 删除视图
    DropView {
        database: String,
        schema: Option<String>,
        view: String,
    },
    /// 创建函数
    CreateFunction { database: String, function: String },
    /// 删除函数
    DropFunction { database: String, function: String },
    /// 创建存储过程
    CreateProcedure { database: String, procedure: String },
    /// 删除存储过程
    DropProcedure { database: String, procedure: String },
    /// 创建触发器
    CreateTrigger {
        database: String,
        schema: Option<String>,
        table: String,
        trigger: String,
    },
    /// 删除触发器
    DropTrigger {
        database: String,
        schema: Option<String>,
        trigger: String,
    },
    /// 创建序列
    CreateSequence {
        database: String,
        schema: Option<String>,
        sequence: String,
    },
    /// 删除序列
    DropSequence {
        database: String,
        schema: Option<String>,
        sequence: String,
    },
    /// 创建 Schema
    CreateSchema { database: String, schema: String },
    /// 删除 Schema
    DropSchema { database: String, schema: String },
}

/// DDL 失效器
///
/// 提供 DDL 语句解析和缓存失效功能，不使用后台任务
pub struct DdlInvalidator {
    cache: Arc<MetadataCacheManager>,
}

impl DdlInvalidator {
    /// 创建新的 DDL 失效器
    pub fn new(cache: Arc<MetadataCacheManager>) -> Self {
        Self { cache }
    }

    /// 处理 DDL 事件（同步失效）
    pub async fn invalidate(&self, connection_id: &str, event: &DdlEvent) {
        Self::handle_event(&self.cache, connection_id, event).await;
    }

    /// 解析 SQL 中的所有 DDL 事件（支持多语句）
    ///
    /// 优先使用 sqlparser AST 解析，失败时 fallback 到字符串匹配。
    /// 如果 SQL 包含 DDL 关键字但无法解析，执行粗粒度兜底（失效整个数据库级缓存）。
    pub fn parse_ddl_events(
        sql: &str,
        current_database: &str,
        current_schema: Option<&str>,
    ) -> Vec<DdlEvent> {
        // 1. 尝试 sqlparser AST 解析
        let dialect = GenericDialect {};
        if let Ok(statements) = Parser::parse_sql(&dialect, sql) {
            let events: Vec<DdlEvent> = statements
                .iter()
                .filter_map(|stmt| {
                    Self::statement_to_ddl_event(stmt, current_database, current_schema)
                })
                .collect();

            if !events.is_empty() {
                return events;
            }

            // AST 解析成功但没有 DDL 事件 → 非 DDL 语句
            if !statements.is_empty() {
                return vec![];
            }
        }

        // 2. AST 解析失败，fallback 到字符串匹配
        if let Some(event) = Self::parse_ddl_event_string(sql, current_database, current_schema) {
            return vec![event];
        }

        // 3. 粗粒度兜底：SQL 包含 DDL 关键字但无法精确解析
        if Self::contains_ddl_keywords(sql) {
            warn!(
                "SQL contains DDL keywords but could not be parsed precisely, \
                 falling back to database-level invalidation: {}",
                &sql[..sql.len().min(100)]
            );
            // 返回一个 CreateDatabase 事件来触发整个数据库级缓存失效
            return vec![DdlEvent::CreateDatabase {
                database: current_database.to_string(),
            }];
        }

        vec![]
    }

    /// 从 SQL 语句解析单个 DDL 事件（向后兼容接口）
    pub fn parse_ddl_event(
        sql: &str,
        current_database: &str,
        current_schema: Option<&str>,
    ) -> Option<DdlEvent> {
        let events = Self::parse_ddl_events(sql, current_database, current_schema);
        events.into_iter().next()
    }

    /// 检查 SQL 是否包含 DDL 关键字
    fn contains_ddl_keywords(sql: &str) -> bool {
        let upper = sql.to_uppercase();
        DDL_KEYWORDS.iter().any(|kw| upper.contains(kw))
    }

    /// 将 sqlparser Statement 转换为 DdlEvent
    fn statement_to_ddl_event(
        stmt: &Statement,
        current_database: &str,
        current_schema: Option<&str>,
    ) -> Option<DdlEvent> {
        match stmt {
            Statement::CreateTable(create_table) => {
                let name = Self::extract_object_name_from_ast(&create_table.name);
                Some(DdlEvent::CreateTable {
                    database: current_database.to_string(),
                    schema: current_schema.map(|s| s.to_string()),
                    table: name,
                })
            }

            Statement::AlterTable(alter_table) => {
                let table = Self::extract_object_name_from_ast(&alter_table.name);
                Some(DdlEvent::AlterTable {
                    database: current_database.to_string(),
                    schema: current_schema.map(|s| s.to_string()),
                    table,
                })
            }

            Statement::Drop {
                object_type, names, ..
            } => {
                use sqlparser::ast::ObjectType;
                let name = names.first().map(Self::extract_object_name_from_ast)?;
                match object_type {
                    ObjectType::Table => Some(DdlEvent::DropTable {
                        database: current_database.to_string(),
                        schema: current_schema.map(|s| s.to_string()),
                        table: name,
                    }),
                    ObjectType::View => Some(DdlEvent::DropView {
                        database: current_database.to_string(),
                        schema: current_schema.map(|s| s.to_string()),
                        view: name,
                    }),
                    ObjectType::Index => Some(DdlEvent::DropIndex {
                        database: current_database.to_string(),
                        schema: current_schema.map(|s| s.to_string()),
                        table: String::new(),
                        index: name,
                    }),
                    ObjectType::Schema => Some(DdlEvent::DropSchema {
                        database: current_database.to_string(),
                        schema: name,
                    }),
                    ObjectType::Database => Some(DdlEvent::DropDatabase { database: name }),
                    ObjectType::Sequence => Some(DdlEvent::DropSequence {
                        database: current_database.to_string(),
                        schema: current_schema.map(|s| s.to_string()),
                        sequence: name,
                    }),
                    _ => None,
                }
            }

            Statement::CreateView(create_view) => {
                let view = Self::extract_object_name_from_ast(&create_view.name);
                Some(DdlEvent::CreateView {
                    database: current_database.to_string(),
                    schema: current_schema.map(|s| s.to_string()),
                    view,
                })
            }

            Statement::CreateIndex(create_index) => {
                let index = create_index
                    .name
                    .as_ref()
                    .map(Self::extract_object_name_from_ast)
                    .unwrap_or_default();
                let table = Self::extract_object_name_from_ast(&create_index.table_name);
                Some(DdlEvent::CreateIndex {
                    database: current_database.to_string(),
                    schema: current_schema.map(|s| s.to_string()),
                    table,
                    index,
                })
            }

            Statement::CreateSchema { schema_name, .. } => {
                let schema = match schema_name {
                    sqlparser::ast::SchemaName::Simple(name) => {
                        Self::extract_object_name_from_ast(name)
                    }
                    sqlparser::ast::SchemaName::NamedAuthorization(name, _) => {
                        Self::extract_object_name_from_ast(name)
                    }
                    sqlparser::ast::SchemaName::UnnamedAuthorization(ident) => ident.value.clone(),
                };
                Some(DdlEvent::CreateSchema {
                    database: current_database.to_string(),
                    schema,
                })
            }

            Statement::CreateDatabase { db_name, .. } => {
                let database = Self::extract_object_name_from_ast(db_name);
                Some(DdlEvent::CreateDatabase { database })
            }

            Statement::CreateFunction(create_function) => {
                let function = Self::extract_object_name_from_ast(&create_function.name);
                Some(DdlEvent::CreateFunction {
                    database: current_database.to_string(),
                    function,
                })
            }

            Statement::DropFunction(drop_function) => {
                let function = drop_function
                    .func_desc
                    .first()
                    .map(|d| Self::extract_object_name_from_ast(&d.name))
                    .unwrap_or_default();
                Some(DdlEvent::DropFunction {
                    database: current_database.to_string(),
                    function,
                })
            }

            Statement::CreateProcedure { name, .. } => {
                let procedure = Self::extract_object_name_from_ast(name);
                Some(DdlEvent::CreateProcedure {
                    database: current_database.to_string(),
                    procedure,
                })
            }

            Statement::DropProcedure { proc_desc, .. } => {
                let procedure = proc_desc
                    .first()
                    .map(|d| Self::extract_object_name_from_ast(&d.name))
                    .unwrap_or_default();
                Some(DdlEvent::DropProcedure {
                    database: current_database.to_string(),
                    procedure,
                })
            }

            Statement::CreateTrigger(create_trigger) => {
                let trigger = Self::extract_object_name_from_ast(&create_trigger.name);
                let table = Self::extract_object_name_from_ast(&create_trigger.table_name);
                Some(DdlEvent::CreateTrigger {
                    database: current_database.to_string(),
                    schema: current_schema.map(|s| s.to_string()),
                    table,
                    trigger,
                })
            }

            Statement::DropTrigger(drop_trigger) => {
                let trigger = Self::extract_object_name_from_ast(&drop_trigger.trigger_name);
                Some(DdlEvent::DropTrigger {
                    database: current_database.to_string(),
                    schema: current_schema.map(|s| s.to_string()),
                    trigger,
                })
            }

            Statement::CreateSequence { name, .. } => {
                let sequence = Self::extract_object_name_from_ast(name);
                Some(DdlEvent::CreateSequence {
                    database: current_database.to_string(),
                    schema: current_schema.map(|s| s.to_string()),
                    sequence,
                })
            }

            Statement::Truncate(truncate) => {
                let table = truncate
                    .table_names
                    .first()
                    .map(|tn| Self::extract_object_name_from_ast(&tn.name))
                    .unwrap_or_default();
                Some(DdlEvent::TruncateTable {
                    database: current_database.to_string(),
                    schema: current_schema.map(|s| s.to_string()),
                    table,
                })
            }

            Statement::RenameTable(operations) => {
                let first = operations.first()?;
                let old_table = Self::extract_object_name_from_ast(&first.old_name);
                let new_table = Self::extract_object_name_from_ast(&first.new_name);
                Some(DdlEvent::RenameTable {
                    database: current_database.to_string(),
                    schema: current_schema.map(|s| s.to_string()),
                    old_table,
                    new_table,
                })
            }

            Statement::AlterView { name, .. } => {
                let view = Self::extract_object_name_from_ast(name);
                Some(DdlEvent::CreateView {
                    database: current_database.to_string(),
                    schema: current_schema.map(|s| s.to_string()),
                    view,
                })
            }

            Statement::AlterIndex { name, .. } => {
                let index = Self::extract_object_name_from_ast(name);
                Some(DdlEvent::DropIndex {
                    database: current_database.to_string(),
                    schema: current_schema.map(|s| s.to_string()),
                    table: String::new(),
                    index,
                })
            }

            _ => None,
        }
    }

    /// 从 sqlparser ObjectName 中提取最后一个标识符（对象名）
    fn extract_object_name_from_ast(name: &sqlparser::ast::ObjectName) -> String {
        name.0
            .last()
            .and_then(|part| match part {
                sqlparser::ast::ObjectNamePart::Identifier(ident) => Some(ident.value.clone()),
                _ => None,
            })
            .unwrap_or_default()
    }

    /// 字符串匹配方式解析 DDL 事件（原有逻辑，作为 fallback）
    fn parse_ddl_event_string(
        sql: &str,
        current_database: &str,
        current_schema: Option<&str>,
    ) -> Option<DdlEvent> {
        let upper = sql.trim().to_uppercase();
        let sql_trimmed = sql.trim();

        // CREATE TABLE
        if upper.starts_with("CREATE TABLE") {
            let table = Self::extract_object_name(sql_trimmed, "CREATE TABLE")?;
            return Some(DdlEvent::CreateTable {
                database: current_database.to_string(),
                schema: current_schema.map(|s| s.to_string()),
                table,
            });
        }

        // ALTER TABLE
        if upper.starts_with("ALTER TABLE") {
            let table = Self::extract_object_name(sql_trimmed, "ALTER TABLE")?;
            return Some(DdlEvent::AlterTable {
                database: current_database.to_string(),
                schema: current_schema.map(|s| s.to_string()),
                table,
            });
        }

        // DROP TABLE
        if upper.starts_with("DROP TABLE") {
            let table = Self::extract_object_name(sql_trimmed, "DROP TABLE")?;
            return Some(DdlEvent::DropTable {
                database: current_database.to_string(),
                schema: current_schema.map(|s| s.to_string()),
                table,
            });
        }

        // TRUNCATE TABLE / TRUNCATE
        if upper.starts_with("TRUNCATE TABLE") {
            let table = Self::extract_object_name(sql_trimmed, "TRUNCATE TABLE")?;
            return Some(DdlEvent::TruncateTable {
                database: current_database.to_string(),
                schema: current_schema.map(|s| s.to_string()),
                table,
            });
        }
        if upper.starts_with("TRUNCATE") && !upper.starts_with("TRUNCATE TABLE") {
            let table = Self::extract_object_name(sql_trimmed, "TRUNCATE")?;
            return Some(DdlEvent::TruncateTable {
                database: current_database.to_string(),
                schema: current_schema.map(|s| s.to_string()),
                table,
            });
        }

        // RENAME TABLE
        if upper.starts_with("RENAME TABLE") {
            let rest = sql_trimmed[12..].trim();
            let parts: Vec<&str> = rest
                .splitn(2, |c: char| {
                    c.eq_ignore_ascii_case(&'T')
                        && rest[rest.find(c).unwrap_or(0)..]
                            .to_uppercase()
                            .starts_with("TO")
                })
                .collect();
            if parts.len() == 2 {
                let old_table =
                    Self::clean_identifier(parts[0].trim().trim_end_matches(char::is_whitespace));
                let new_table = Self::clean_identifier(
                    parts[1]
                        .trim_start_matches(|c: char| {
                            c.eq_ignore_ascii_case(&'O') || c.is_whitespace()
                        })
                        .trim(),
                );
                return Some(DdlEvent::RenameTable {
                    database: current_database.to_string(),
                    schema: current_schema.map(|s| s.to_string()),
                    old_table,
                    new_table,
                });
            }
        }

        // CREATE DATABASE
        if upper.starts_with("CREATE DATABASE") {
            let database = Self::extract_object_name(sql_trimmed, "CREATE DATABASE")?;
            return Some(DdlEvent::CreateDatabase { database });
        }

        // DROP DATABASE
        if upper.starts_with("DROP DATABASE") {
            let database = Self::extract_object_name(sql_trimmed, "DROP DATABASE")?;
            return Some(DdlEvent::DropDatabase { database });
        }

        // CREATE INDEX
        if upper.starts_with("CREATE INDEX") || upper.starts_with("CREATE UNIQUE INDEX") {
            if let Some(on_pos) = upper.find(" ON ") {
                let after_on = sql_trimmed[on_pos + 4..].trim();
                let table = after_on
                    .split(|c: char| c.is_whitespace() || c == '(')
                    .next()
                    .map(Self::clean_identifier)?;

                let index_start = if upper.starts_with("CREATE UNIQUE INDEX") {
                    "CREATE UNIQUE INDEX".len()
                } else {
                    "CREATE INDEX".len()
                };
                let index_part = sql_trimmed[index_start..on_pos].trim();
                let index = Self::extract_first_identifier(index_part)?;

                return Some(DdlEvent::CreateIndex {
                    database: current_database.to_string(),
                    schema: current_schema.map(|s| s.to_string()),
                    table,
                    index,
                });
            }
        }

        // DROP INDEX
        if upper.starts_with("DROP INDEX") {
            let index = Self::extract_object_name(sql_trimmed, "DROP INDEX")?;
            return Some(DdlEvent::DropIndex {
                database: current_database.to_string(),
                schema: current_schema.map(|s| s.to_string()),
                table: String::new(),
                index,
            });
        }

        // CREATE VIEW
        if upper.starts_with("CREATE VIEW") || upper.starts_with("CREATE OR REPLACE VIEW") {
            let prefix = if upper.starts_with("CREATE OR REPLACE VIEW") {
                "CREATE OR REPLACE VIEW"
            } else {
                "CREATE VIEW"
            };
            let view = Self::extract_object_name(sql_trimmed, prefix)?;
            return Some(DdlEvent::CreateView {
                database: current_database.to_string(),
                schema: current_schema.map(|s| s.to_string()),
                view,
            });
        }

        // DROP VIEW
        if upper.starts_with("DROP VIEW") {
            let view = Self::extract_object_name(sql_trimmed, "DROP VIEW")?;
            return Some(DdlEvent::DropView {
                database: current_database.to_string(),
                schema: current_schema.map(|s| s.to_string()),
                view,
            });
        }

        // CREATE FUNCTION
        if upper.starts_with("CREATE FUNCTION") || upper.starts_with("CREATE OR REPLACE FUNCTION") {
            let prefix = if upper.starts_with("CREATE OR REPLACE FUNCTION") {
                "CREATE OR REPLACE FUNCTION"
            } else {
                "CREATE FUNCTION"
            };
            let function = Self::extract_object_name(sql_trimmed, prefix)?;
            return Some(DdlEvent::CreateFunction {
                database: current_database.to_string(),
                function,
            });
        }

        // DROP FUNCTION
        if upper.starts_with("DROP FUNCTION") {
            let function = Self::extract_object_name(sql_trimmed, "DROP FUNCTION")?;
            return Some(DdlEvent::DropFunction {
                database: current_database.to_string(),
                function,
            });
        }

        // CREATE PROCEDURE
        if upper.starts_with("CREATE PROCEDURE") || upper.starts_with("CREATE OR REPLACE PROCEDURE")
        {
            let prefix = if upper.starts_with("CREATE OR REPLACE PROCEDURE") {
                "CREATE OR REPLACE PROCEDURE"
            } else {
                "CREATE PROCEDURE"
            };
            let procedure = Self::extract_object_name(sql_trimmed, prefix)?;
            return Some(DdlEvent::CreateProcedure {
                database: current_database.to_string(),
                procedure,
            });
        }

        // DROP PROCEDURE
        if upper.starts_with("DROP PROCEDURE") {
            let procedure = Self::extract_object_name(sql_trimmed, "DROP PROCEDURE")?;
            return Some(DdlEvent::DropProcedure {
                database: current_database.to_string(),
                procedure,
            });
        }

        // CREATE TRIGGER
        if upper.starts_with("CREATE TRIGGER") || upper.starts_with("CREATE OR REPLACE TRIGGER") {
            let prefix = if upper.starts_with("CREATE OR REPLACE TRIGGER") {
                "CREATE OR REPLACE TRIGGER"
            } else {
                "CREATE TRIGGER"
            };
            let trigger = Self::extract_object_name(sql_trimmed, prefix)?;
            let table = if let Some(on_pos) = upper.find(" ON ") {
                let after_on = sql_trimmed[on_pos + 4..].trim();
                after_on
                    .split(|c: char| c.is_whitespace() || c == '(')
                    .next()
                    .map(Self::clean_identifier)
                    .unwrap_or_default()
            } else {
                String::new()
            };

            return Some(DdlEvent::CreateTrigger {
                database: current_database.to_string(),
                schema: current_schema.map(|s| s.to_string()),
                table,
                trigger,
            });
        }

        // DROP TRIGGER
        if upper.starts_with("DROP TRIGGER") {
            let trigger = Self::extract_object_name(sql_trimmed, "DROP TRIGGER")?;
            return Some(DdlEvent::DropTrigger {
                database: current_database.to_string(),
                schema: current_schema.map(|s| s.to_string()),
                trigger,
            });
        }

        // CREATE SEQUENCE
        if upper.starts_with("CREATE SEQUENCE") {
            let sequence = Self::extract_object_name(sql_trimmed, "CREATE SEQUENCE")?;
            return Some(DdlEvent::CreateSequence {
                database: current_database.to_string(),
                schema: current_schema.map(|s| s.to_string()),
                sequence,
            });
        }

        // DROP SEQUENCE
        if upper.starts_with("DROP SEQUENCE") {
            let sequence = Self::extract_object_name(sql_trimmed, "DROP SEQUENCE")?;
            return Some(DdlEvent::DropSequence {
                database: current_database.to_string(),
                schema: current_schema.map(|s| s.to_string()),
                sequence,
            });
        }

        // CREATE SCHEMA
        if upper.starts_with("CREATE SCHEMA") {
            let schema = Self::extract_object_name(sql_trimmed, "CREATE SCHEMA")?;
            return Some(DdlEvent::CreateSchema {
                database: current_database.to_string(),
                schema,
            });
        }

        // DROP SCHEMA
        if upper.starts_with("DROP SCHEMA") {
            let schema = Self::extract_object_name(sql_trimmed, "DROP SCHEMA")?;
            return Some(DdlEvent::DropSchema {
                database: current_database.to_string(),
                schema,
            });
        }

        None
    }

    /// 从 SQL 中提取对象名称
    fn extract_object_name(sql: &str, prefix: &str) -> Option<String> {
        let upper = sql.to_uppercase();
        let start = upper.find(&prefix.to_uppercase())? + prefix.len();
        let rest = sql[start..].trim();

        // 处理 IF EXISTS / IF NOT EXISTS
        let rest = if rest.to_uppercase().starts_with("IF EXISTS") {
            rest[9..].trim()
        } else if rest.to_uppercase().starts_with("IF NOT EXISTS") {
            rest[13..].trim()
        } else {
            rest
        };

        Self::extract_first_identifier(rest)
    }

    /// 提取第一个标识符
    fn extract_first_identifier(s: &str) -> Option<String> {
        let s = s.trim();

        // 处理带引号的标识符
        if s.starts_with('`') {
            // MySQL 反引号
            let end = s[1..].find('`')?;
            return Some(s[1..end + 1].to_string());
        }
        if s.starts_with('"') {
            // 双引号
            let end = s[1..].find('"')?;
            return Some(s[1..end + 1].to_string());
        }
        if s.starts_with('[') {
            // SQL Server 方括号
            let end = s[1..].find(']')?;
            return Some(s[1..end + 1].to_string());
        }

        // 普通标识符
        let name: String = s
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '.')
            .collect();

        if name.is_empty() {
            None
        } else {
            // 处理 schema.name 格式，取最后一部分
            let parts: Vec<&str> = name.split('.').collect();
            Some(parts.last()?.to_string())
        }
    }

    /// 清理标识符（去除引号）
    fn clean_identifier(s: &str) -> String {
        let s = s.trim();
        let s = s.trim_matches('`');
        let s = s.trim_matches('"');
        let s = s.trim_matches('[').trim_matches(']');
        let s = s.trim_end_matches(|c: char| c.is_whitespace() || c == '(' || c == ';');

        // 处理 schema.name 格式
        if let Some(pos) = s.rfind('.') {
            s[pos + 1..].to_string()
        } else {
            s.to_string()
        }
    }

    /// 处理单个 DDL 事件
    async fn handle_event(cache: &MetadataCacheManager, connection_id: &str, event: &DdlEvent) {
        debug!("Processing DDL event for {}: {:?}", connection_id, event);

        match event {
            DdlEvent::CreateTable {
                database,
                schema,
                table,
            }
            | DdlEvent::AlterTable {
                database,
                schema,
                table,
            }
            | DdlEvent::DropTable {
                database,
                schema,
                table,
            }
            | DdlEvent::TruncateTable {
                database,
                schema,
                table,
            } => {
                // 失效表结构缓存
                cache
                    .invalidate_table(connection_id, database, schema.as_deref(), table)
                    .await;
                // 失效表列表缓存
                cache
                    .invalidate_table_list(connection_id, database, schema.as_deref())
                    .await;
            }

            DdlEvent::RenameTable {
                database,
                schema,
                old_table,
                new_table,
            } => {
                // 失效新旧表的缓存
                cache
                    .invalidate_table(connection_id, database, schema.as_deref(), old_table)
                    .await;
                cache
                    .invalidate_table(connection_id, database, schema.as_deref(), new_table)
                    .await;
                // 失效表列表缓存
                cache
                    .invalidate_table_list(connection_id, database, schema.as_deref())
                    .await;
            }

            DdlEvent::CreateDatabase { database } | DdlEvent::DropDatabase { database } => {
                // 失效数据库列表缓存
                let key = CacheKey::databases(connection_id);
                cache.invalidate(&key).await;
                // 失效该数据库下所有缓存
                cache.invalidate_database(connection_id, database).await;
            }

            DdlEvent::CreateIndex {
                database,
                schema,
                table,
                ..
            }
            | DdlEvent::DropIndex {
                database,
                schema,
                table,
                ..
            } => {
                // 失效索引缓存
                let key = CacheKey::indexes(connection_id, database, schema.as_deref(), table);
                cache.invalidate(&key).await;
            }

            DdlEvent::CreateView {
                database, schema, ..
            }
            | DdlEvent::DropView {
                database, schema, ..
            } => {
                // 失效视图列表缓存
                let key = CacheKey::views(connection_id, database, schema.as_deref());
                cache.invalidate(&key).await;
            }

            DdlEvent::CreateFunction { database, .. } | DdlEvent::DropFunction { database, .. } => {
                // 失效函数列表缓存
                let key = CacheKey::functions(connection_id, database);
                cache.invalidate(&key).await;
            }

            DdlEvent::CreateProcedure { database, .. }
            | DdlEvent::DropProcedure { database, .. } => {
                // 失效存储过程列表缓存
                let key = CacheKey::procedures(connection_id, database);
                cache.invalidate(&key).await;
            }

            DdlEvent::CreateTrigger {
                database,
                schema,
                table,
                ..
            } => {
                // 失效触发器缓存
                let key = CacheKey::triggers(connection_id, database);
                cache.invalidate(&key).await;
                if !table.is_empty() {
                    let key =
                        CacheKey::table_triggers(connection_id, database, schema.as_deref(), table);
                    cache.invalidate(&key).await;
                }
            }

            DdlEvent::DropTrigger { database, .. } => {
                // 失效触发器缓存
                let key = CacheKey::triggers(connection_id, database);
                cache.invalidate(&key).await;
            }

            DdlEvent::CreateSequence {
                database, schema, ..
            }
            | DdlEvent::DropSequence {
                database, schema, ..
            } => {
                // 失效序列列表缓存
                let key = CacheKey::sequences(connection_id, database, schema.as_deref());
                cache.invalidate(&key).await;
            }

            DdlEvent::CreateSchema { database, .. } | DdlEvent::DropSchema { database, .. } => {
                // 失效 Schema 列表缓存
                let key = CacheKey::schemas(connection_id, database);
                cache.invalidate(&key).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_create_table() {
        let event = DdlInvalidator::parse_ddl_event(
            "CREATE TABLE users (id INT PRIMARY KEY)",
            "mydb",
            None,
        );
        assert!(matches!(event, Some(DdlEvent::CreateTable { table, .. }) if table == "users"));

        let event = DdlInvalidator::parse_ddl_event(
            "CREATE TABLE IF NOT EXISTS `orders` (id INT)",
            "mydb",
            None,
        );
        assert!(matches!(event, Some(DdlEvent::CreateTable { table, .. }) if table == "orders"));
    }

    #[test]
    fn test_parse_alter_table() {
        let event = DdlInvalidator::parse_ddl_event(
            "ALTER TABLE users ADD COLUMN email VARCHAR(255)",
            "mydb",
            None,
        );
        assert!(matches!(event, Some(DdlEvent::AlterTable { table, .. }) if table == "users"));
    }

    #[test]
    fn test_parse_drop_table() {
        let event = DdlInvalidator::parse_ddl_event("DROP TABLE IF EXISTS users", "mydb", None);
        assert!(matches!(event, Some(DdlEvent::DropTable { table, .. }) if table == "users"));
    }

    #[test]
    fn test_parse_truncate_table() {
        let event = DdlInvalidator::parse_ddl_event("TRUNCATE TABLE users", "mydb", None);
        assert!(matches!(event, Some(DdlEvent::TruncateTable { table, .. }) if table == "users"));
    }

    #[test]
    fn test_parse_create_database() {
        let event = DdlInvalidator::parse_ddl_event("CREATE DATABASE testdb", "mydb", None);
        assert!(
            matches!(event, Some(DdlEvent::CreateDatabase { database }) if database == "testdb")
        );
    }

    #[test]
    fn test_parse_create_index() {
        let event = DdlInvalidator::parse_ddl_event(
            "CREATE INDEX idx_users_email ON users (email)",
            "mydb",
            None,
        );
        assert!(
            matches!(event, Some(DdlEvent::CreateIndex { table, index, .. }) if table == "users" && index == "idx_users_email")
        );

        let event = DdlInvalidator::parse_ddl_event(
            "CREATE UNIQUE INDEX idx_email ON users (email)",
            "mydb",
            None,
        );
        assert!(
            matches!(event, Some(DdlEvent::CreateIndex { table, index, .. }) if table == "users" && index == "idx_email")
        );
    }

    #[test]
    fn test_parse_create_view() {
        let event = DdlInvalidator::parse_ddl_event(
            "CREATE VIEW active_users AS SELECT * FROM users WHERE active = 1",
            "mydb",
            None,
        );
        assert!(matches!(event, Some(DdlEvent::CreateView { view, .. }) if view == "active_users"));

        let event = DdlInvalidator::parse_ddl_event(
            "CREATE OR REPLACE VIEW active_users AS SELECT * FROM users",
            "mydb",
            None,
        );
        assert!(matches!(event, Some(DdlEvent::CreateView { view, .. }) if view == "active_users"));
    }

    #[test]
    fn test_parse_create_function() {
        let event = DdlInvalidator::parse_ddl_event(
            "CREATE FUNCTION get_user_count() RETURNS INT BEGIN RETURN 0; END",
            "mydb",
            None,
        );
        assert!(
            matches!(event, Some(DdlEvent::CreateFunction { function, .. }) if function == "get_user_count")
        );
    }

    #[test]
    fn test_parse_create_trigger() {
        let event = DdlInvalidator::parse_ddl_event(
            "CREATE TRIGGER user_insert_trigger AFTER INSERT ON users FOR EACH ROW BEGIN END",
            "mydb",
            None,
        );
        assert!(
            matches!(event, Some(DdlEvent::CreateTrigger { trigger, table, .. }) if trigger == "user_insert_trigger" && table == "users")
        );
    }

    #[test]
    fn test_parse_with_schema() {
        let event = DdlInvalidator::parse_ddl_event(
            "CREATE TABLE public.users (id INT)",
            "mydb",
            Some("public"),
        );
        assert!(
            matches!(event, Some(DdlEvent::CreateTable { table, schema, .. }) if table == "users" && schema == Some("public".to_string()))
        );
    }

    #[test]
    fn test_parse_non_ddl() {
        let event = DdlInvalidator::parse_ddl_event("SELECT * FROM users", "mydb", None);
        assert!(event.is_none());

        let event =
            DdlInvalidator::parse_ddl_event("INSERT INTO users VALUES (1, 'test')", "mydb", None);
        assert!(event.is_none());

        let event = DdlInvalidator::parse_ddl_event("UPDATE users SET name = 'test'", "mydb", None);
        assert!(event.is_none());
    }

    #[test]
    fn test_extract_identifier_with_quotes() {
        assert_eq!(
            DdlInvalidator::extract_first_identifier("`users`"),
            Some("users".to_string())
        );
        assert_eq!(
            DdlInvalidator::extract_first_identifier("\"users\""),
            Some("users".to_string())
        );
        assert_eq!(
            DdlInvalidator::extract_first_identifier("[users]"),
            Some("users".to_string())
        );
        assert_eq!(
            DdlInvalidator::extract_first_identifier("public.users"),
            Some("users".to_string())
        );
    }

    #[test]
    fn test_parse_multi_statement() {
        let sql = "CREATE TABLE users (id INT); CREATE TABLE orders (id INT);";
        let events = DdlInvalidator::parse_ddl_events(sql, "mydb", None);
        assert_eq!(events.len(), 2);
        assert!(matches!(&events[0], DdlEvent::CreateTable { table, .. } if table == "users"));
        assert!(matches!(&events[1], DdlEvent::CreateTable { table, .. } if table == "orders"));
    }

    #[test]
    fn test_parse_mixed_statements() {
        let sql = "SELECT 1; CREATE TABLE users (id INT); INSERT INTO users VALUES (1);";
        let events = DdlInvalidator::parse_ddl_events(sql, "mydb", None);
        assert_eq!(events.len(), 1);
        assert!(matches!(&events[0], DdlEvent::CreateTable { table, .. } if table == "users"));
    }

    #[test]
    fn test_parse_drop_with_sqlparser() {
        let event = DdlInvalidator::parse_ddl_event("DROP VIEW IF EXISTS my_view", "mydb", None);
        assert!(matches!(event, Some(DdlEvent::DropView { view, .. }) if view == "my_view"));

        let event =
            DdlInvalidator::parse_ddl_event("DROP SCHEMA IF EXISTS my_schema", "mydb", None);
        assert!(
            matches!(event, Some(DdlEvent::DropSchema { schema, .. }) if schema == "my_schema")
        );
    }
}
