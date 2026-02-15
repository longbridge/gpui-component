use sqlformat::{format, FormatOptions, QueryParams};

/// SQL 美化：将 SQL 格式化为可读性更好的多行形式
pub fn format_sql(sql: &str) -> String {
    format(sql, &QueryParams::None, &FormatOptions::default())
}

/// SQL 压缩：将 SQL 压缩为单行形式
pub fn compress_sql(sql: &str) -> String {
    sql.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_sql() {
        let sql = "select id, name from users where id = 1";
        let formatted = format_sql(sql);
        let formatted_upper = formatted.to_uppercase();
        assert!(formatted_upper.contains("SELECT"));
        assert!(formatted_upper.contains("FROM"));
        assert!(formatted_upper.contains("WHERE"));
    }

    #[test]
    fn test_compress_sql() {
        let sql = "SELECT\n  id,\n  name\nFROM\n  users\nWHERE\n  id = 1";
        let compressed = compress_sql(sql);
        assert_eq!(compressed, "SELECT id, name FROM users WHERE id = 1");
    }
}
