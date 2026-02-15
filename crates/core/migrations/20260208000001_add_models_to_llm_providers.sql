-- Add api_version/models/is_default to llm_providers
ALTER TABLE llm_providers ADD COLUMN api_version TEXT;
ALTER TABLE llm_providers ADD COLUMN models TEXT;
ALTER TABLE llm_providers ADD COLUMN is_default INTEGER NOT NULL DEFAULT 0;
