use gpui::SharedString;

/// 复制格式枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyFormat {
    /// Tab 分隔值（默认，与 Excel 兼容）
    Tsv,
    /// 逗号分隔值
    Csv,
    /// JSON 数组格式
    Json,
    /// Markdown 表格
    Markdown,
    /// SQL INSERT 语句
    SqlInsert,
    /// SQL UPDATE 语句（需要主键）
    SqlUpdate,
    /// SQL DELETE 语句（需要主键）
    SqlDelete,
    /// SQL IN 子句（单列时）
    SqlIn,
}

/// 表格元数据（用于生成 SQL）
#[derive(Debug, Clone, Default)]
pub struct TableMetadata {
    /// 表名
    pub table_name: SharedString,
    /// 列名列表
    pub column_names: Vec<SharedString>,
    /// 主键列索引（可能有多个复合主键）
    pub primary_key_indices: Vec<usize>,
}

impl TableMetadata {
    pub fn new(table_name: impl Into<SharedString>) -> Self {
        Self {
            table_name: table_name.into(),
            column_names: Vec::new(),
            primary_key_indices: Vec::new(),
        }
    }

    pub fn with_columns(mut self, columns: Vec<impl Into<SharedString>>) -> Self {
        self.column_names = columns.into_iter().map(|c| c.into()).collect();
        self
    }

    pub fn with_primary_keys(mut self, indices: Vec<usize>) -> Self {
        self.primary_key_indices = indices;
        self
    }
}

/// 格式化器
pub struct CopyFormatter;

impl CopyFormatter {
    /// 格式化数据为指定格式
    pub fn format(
        format: CopyFormat,
        data: &[Vec<String>],
        columns: &[SharedString],
        metadata: &TableMetadata,
    ) -> String {
        match format {
            CopyFormat::Tsv => Self::format_tsv(data),
            CopyFormat::Csv => Self::format_csv(data),
            CopyFormat::Json => Self::format_json(data, columns),
            CopyFormat::Markdown => Self::format_markdown(data, columns),
            CopyFormat::SqlInsert => Self::format_sql_insert(data, columns, metadata),
            CopyFormat::SqlUpdate => Self::format_sql_update(data, columns, metadata),
            CopyFormat::SqlDelete => Self::format_sql_delete(data, columns, metadata),
            CopyFormat::SqlIn => Self::format_sql_in(data, columns),
        }
    }

    /// TSV 格式（Tab 分隔）
    fn format_tsv(data: &[Vec<String>]) -> String {
        data.iter()
            .map(|row| row.join("\t"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// CSV 格式
    fn format_csv(data: &[Vec<String>]) -> String {
        data.iter()
            .map(|row| {
                row.iter()
                    .map(|cell| Self::escape_csv_field(cell))
                    .collect::<Vec<_>>()
                    .join(",")
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// JSON 格式
    fn format_json(data: &[Vec<String>], columns: &[SharedString]) -> String {
        let rows: Vec<String> = data
            .iter()
            .map(|row| {
                let fields: Vec<String> = row
                    .iter()
                    .enumerate()
                    .map(|(i, value)| {
                        let col_name = columns.get(i).map(|s| s.as_ref()).unwrap_or("_");
                        format!("    \"{}\": {}", col_name, Self::to_json_value(value))
                    })
                    .collect();
                format!("  {{\n{}\n  }}", fields.join(",\n"))
            })
            .collect();
        format!("[\n{}\n]", rows.join(",\n"))
    }

    /// Markdown 表格格式
    fn format_markdown(data: &[Vec<String>], columns: &[SharedString]) -> String {
        if data.is_empty() {
            return String::new();
        }

        let col_count = data.first().map(|r| r.len()).unwrap_or(0);

        // 表头
        let header: Vec<&str> = (0..col_count)
            .map(|i| columns.get(i).map(|s| s.as_ref()).unwrap_or("-"))
            .collect();

        let mut result = format!("| {} |", header.join(" | "));
        result.push('\n');

        // 分隔线
        let separator: Vec<&str> = (0..col_count).map(|_| "---").collect();
        result.push_str(&format!("| {} |", separator.join(" | ")));
        result.push('\n');

        // 数据行
        for row in data {
            let escaped: Vec<String> = row
                .iter()
                .map(|cell| cell.replace('|', "\\|"))
                .collect();
            result.push_str(&format!("| {} |", escaped.join(" | ")));
            result.push('\n');
        }

        result.trim_end().to_string()
    }

    /// SQL INSERT 语句
    fn format_sql_insert(
        data: &[Vec<String>],
        columns: &[SharedString],
        metadata: &TableMetadata,
    ) -> String {
        if data.is_empty() {
            return String::new();
        }

        let table_name = if metadata.table_name.is_empty() {
            "table_name"
        } else {
            metadata.table_name.as_ref()
        };

        let col_count = data.first().map(|r| r.len()).unwrap_or(0);
        let col_names: Vec<&str> = (0..col_count)
            .map(|i| columns.get(i).map(|s| s.as_ref()).unwrap_or("col"))
            .collect();

        let values: Vec<String> = data
            .iter()
            .map(|row| {
                let vals: Vec<String> = row.iter().map(|v| Self::to_sql_value(v)).collect();
                format!("({})", vals.join(", "))
            })
            .collect();

        format!(
            "INSERT INTO {} ({}) VALUES\n{};",
            table_name,
            col_names.join(", "),
            values.join(",\n")
        )
    }

    /// SQL UPDATE 语句
    fn format_sql_update(
        data: &[Vec<String>],
        columns: &[SharedString],
        metadata: &TableMetadata,
    ) -> String {
        if data.is_empty() {
            return String::new();
        }

        let table_name = if metadata.table_name.is_empty() {
            "table_name"
        } else {
            metadata.table_name.as_ref()
        };

        let pk_indices = if metadata.primary_key_indices.is_empty() {
            vec![0] // 默认第一列为主键
        } else {
            metadata.primary_key_indices.clone()
        };

        let statements: Vec<String> = data
            .iter()
            .map(|row| {
                // SET 子句（非主键列）
                let set_parts: Vec<String> = row
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| !pk_indices.contains(i))
                    .map(|(i, v)| {
                        let col_name = columns.get(i).map(|s| s.as_ref()).unwrap_or("col");
                        format!("{} = {}", col_name, Self::to_sql_value(v))
                    })
                    .collect();

                // WHERE 子句（主键列）
                let where_parts: Vec<String> = pk_indices
                    .iter()
                    .filter_map(|&i| {
                        let col_name = columns.get(i).map(|s| s.as_ref())?;
                        let value = row.get(i)?;
                        Some(format!("{} = {}", col_name, Self::to_sql_value(value)))
                    })
                    .collect();

                if set_parts.is_empty() || where_parts.is_empty() {
                    return String::new();
                }

                format!(
                    "UPDATE {} SET {} WHERE {};",
                    table_name,
                    set_parts.join(", "),
                    where_parts.join(" AND ")
                )
            })
            .filter(|s| !s.is_empty())
            .collect();

        statements.join("\n")
    }

    /// SQL DELETE 语句
    fn format_sql_delete(
        data: &[Vec<String>],
        columns: &[SharedString],
        metadata: &TableMetadata,
    ) -> String {
        if data.is_empty() {
            return String::new();
        }

        let table_name = if metadata.table_name.is_empty() {
            "table_name"
        } else {
            metadata.table_name.as_ref()
        };

        let pk_indices = if metadata.primary_key_indices.is_empty() {
            vec![0] // 默认第一列为主键
        } else {
            metadata.primary_key_indices.clone()
        };

        let statements: Vec<String> = data
            .iter()
            .map(|row| {
                let where_parts: Vec<String> = pk_indices
                    .iter()
                    .filter_map(|&i| {
                        let col_name = columns.get(i).map(|s| s.as_ref())?;
                        let value = row.get(i)?;
                        Some(format!("{} = {}", col_name, Self::to_sql_value(value)))
                    })
                    .collect();

                if where_parts.is_empty() {
                    return String::new();
                }

                format!(
                    "DELETE FROM {} WHERE {};",
                    table_name,
                    where_parts.join(" AND ")
                )
            })
            .filter(|s| !s.is_empty())
            .collect();

        statements.join("\n")
    }

    /// SQL IN 子句（适用于单列）
    fn format_sql_in(data: &[Vec<String>], columns: &[SharedString]) -> String {
        if data.is_empty() {
            return String::new();
        }

        // 如果是单列，生成 IN 子句
        if data.first().map(|r| r.len()).unwrap_or(0) == 1 {
            let col_name = columns.first().map(|s| s.as_ref()).unwrap_or("column");
            let values: Vec<String> = data
                .iter()
                .filter_map(|row| row.first())
                .map(|v| Self::to_sql_value(v))
                .collect();
            return format!("{} IN ({})", col_name, values.join(", "));
        }

        // 多列时生成 OR 条件
        let conditions: Vec<String> = data
            .iter()
            .map(|row| {
                let parts: Vec<String> = row
                    .iter()
                    .enumerate()
                    .map(|(i, v)| {
                        let col_name = columns.get(i).map(|s| s.as_ref()).unwrap_or("col");
                        format!("{} = {}", col_name, Self::to_sql_value(v))
                    })
                    .collect();
                format!("({})", parts.join(" AND "))
            })
            .collect();

        conditions.join(" OR\n")
    }

    // === 辅助方法 ===

    /// 转义 CSV 字段
    fn escape_csv_field(field: &str) -> String {
        if field.contains(',') || field.contains('"') || field.contains('\n') {
            format!("\"{}\"", field.replace('"', "\"\""))
        } else {
            field.to_string()
        }
    }

    /// 转换为 JSON 值
    fn to_json_value(value: &str) -> String {
        // 尝试解析为数字
        if value.parse::<i64>().is_ok() || value.parse::<f64>().is_ok() {
            return value.to_string();
        }
        // 布尔值
        if value == "true" || value == "false" {
            return value.to_string();
        }
        // NULL
        if value.is_empty() || value.eq_ignore_ascii_case("null") {
            return "null".to_string();
        }
        // 字符串（转义）
        let escaped = value
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t");
        format!("\"{}\"", escaped)
    }

    /// 转换为 SQL 值
    fn to_sql_value(value: &str) -> String {
        // NULL
        if value.is_empty() || value.eq_ignore_ascii_case("null") {
            return "NULL".to_string();
        }
        // 数字
        if value.parse::<i64>().is_ok() || value.parse::<f64>().is_ok() {
            return value.to_string();
        }
        // 字符串（转义单引号）
        format!("'{}'", value.replace('\'', "''"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_csv() {
        let data = vec![
            vec!["a".to_string(), "b,c".to_string()],
            vec!["d".to_string(), "e\"f".to_string()],
        ];
        let result = CopyFormatter::format_csv(&data);
        assert_eq!(result, "a,\"b,c\"\nd,\"e\"\"f\"");
    }

    #[test]
    fn test_format_sql_insert() {
        let data = vec![
            vec!["1".to_string(), "Alice".to_string()],
            vec!["2".to_string(), "Bob".to_string()],
        ];
        let columns = vec!["id".into(), "name".into()];
        let metadata = TableMetadata::new("users");
        let result = CopyFormatter::format_sql_insert(&data, &columns, &metadata);
        assert!(result.contains("INSERT INTO users"));
        assert!(result.contains("(1, 'Alice')"));
    }

    #[test]
    fn test_to_sql_value() {
        assert_eq!(CopyFormatter::to_sql_value("123"), "123");
        assert_eq!(CopyFormatter::to_sql_value("hello"), "'hello'");
        assert_eq!(CopyFormatter::to_sql_value("it's"), "'it''s'");
        assert_eq!(CopyFormatter::to_sql_value(""), "NULL");
    }
}
