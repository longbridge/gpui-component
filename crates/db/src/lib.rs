rust_i18n::i18n!("locales", fallback = "en");

pub mod cache;
pub mod cache_manager;
pub mod connection;
pub mod ddl_invalidator;
pub mod executor;
pub mod import_export;
pub mod manager;
pub mod metadata_cache;
pub mod plugin;
pub mod rustls_provider;
pub mod sql_format;
pub mod ssh_tunnel;
pub mod streaming_parser;
pub mod types;

// Database implementations
pub mod clickhouse;
pub mod duckdb;
pub mod mssql;
pub mod mysql;
pub mod oracle;
pub mod postgresql;
pub mod sql_editor;
pub mod sqlite;

// Re-exports
pub use cache::*;
pub use cache_manager::*;
pub use connection::*;
pub use ddl_invalidator::*;
pub use executor::*;
pub use import_export::*;
pub use manager::*;
pub use metadata_cache::*;
pub use plugin::*;
pub use rustls_provider::*;
pub use sql_format::*;
pub use ssh_tunnel::*;
pub use streaming_parser::*;
pub use types::*;

pub fn truncate_str(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}
