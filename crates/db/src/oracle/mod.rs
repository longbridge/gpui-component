pub mod connection;
pub mod plugin;

pub use connection::OracleDbConnection;
pub use plugin::OraclePlugin;

/// 检测本地 Oracle Client 是否可用，并返回客户端版本字符串。
pub fn detect_local_client_version() -> Result<String, String> {
    oracle::Version::client()
        .map(|version| version.to_string())
        .map_err(|err| err.to_string())
}
