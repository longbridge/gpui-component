use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tokio::time;

// Version number of the HelloWorld struct
const VERSION: &str = "1.0.0";

/// HelloWorld struct provides greeting functionality with configuration options
///
/// # Features
/// - Async greetings with customizable names
/// - Configuration management via HashMap
/// - Report generation
/// - Error handling with custom error types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelloWorld {
    name: String,
    #[serde(skip)]
    options: HashMap<String, serde_json::Value>,
    created_at: chrono::DateTime<chrono::Utc>,
}
