pub struct MCPConfig {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub settings: std::collections::HashMap<String, String>,
}

impl MCPConfig {
    pub fn new(id: String, name: String, description: String, version: String) -> Self {
        Self {
            id,
            name,
            description,
            version,
            settings: std::collections::HashMap::new(),
        }
    }

    pub fn set_setting(&mut self, key: String, value: String) {
        self.settings.insert(key, value);
    }

    pub fn get_setting(&self, key: &str) -> Option<&String> {
        self.settings.get(key)
    }
}