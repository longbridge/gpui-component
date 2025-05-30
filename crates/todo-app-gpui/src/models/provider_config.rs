pub struct ProviderConfig {
    pub id: String,
    pub name: String,
    pub endpoint: String,
    pub api_key: Option<String>,
}

impl ProviderConfig {
    pub fn new(id: String, name: String, endpoint: String, api_key: Option<String>) -> Self {
        Self {
            id,
            name,
            endpoint,
            api_key,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.name.is_empty() {
            return Err("Provider name cannot be empty".to_string());
        }
        if self.endpoint.is_empty() {
            return Err("Endpoint cannot be empty".to_string());
        }
        Ok(())
    }
}