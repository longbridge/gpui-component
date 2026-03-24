use crate::DatabasePlugin;
use crate::import_export::ImportConfig;

pub mod csv;
pub mod json;
pub mod sql;
pub mod txt;
pub mod xml;

pub use csv::CsvFormatHandler;
pub use json::JsonFormatHandler;
pub use sql::SqlFormatHandler;
pub use txt::TxtFormatHandler;
pub use xml::XmlFormatHandler;

pub(super) fn format_import_table_reference(
    plugin: &dyn DatabasePlugin,
    config: &ImportConfig,
    table: &str,
) -> String {
    plugin.format_table_reference(&config.database, config.schema.as_deref(), table)
}

#[cfg(test)]
mod tests {
    use super::format_import_table_reference;
    use crate::import_export::ImportConfig;
    use crate::mssql::MsSqlPlugin;
    use crate::mysql::MySqlPlugin;

    #[test]
    fn test_format_import_table_reference_uses_database_for_mysql() {
        let plugin = MySqlPlugin::new();
        let config = ImportConfig {
            database: "analytics".to_string(),
            table: Some("orders".to_string()),
            ..ImportConfig::default()
        };

        let table_ref = format_import_table_reference(&plugin, &config, "orders");

        assert_eq!(table_ref, "`analytics`.`orders`");
    }

    #[test]
    fn test_format_import_table_reference_uses_schema_for_mssql() {
        let plugin = MsSqlPlugin::new();
        let config = ImportConfig {
            database: "warehouse".to_string(),
            schema: Some("sales".to_string()),
            table: Some("orders".to_string()),
            ..ImportConfig::default()
        };

        let table_ref = format_import_table_reference(&plugin, &config, "orders");

        assert_eq!(table_ref, "[warehouse].[sales].[orders]");
    }
}
