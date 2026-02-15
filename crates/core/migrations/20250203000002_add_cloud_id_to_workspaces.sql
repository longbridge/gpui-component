-- Add cloud_id column to workspaces table for cloud sync
ALTER TABLE workspaces ADD COLUMN cloud_id TEXT;

-- Add index for cloud_id lookups
CREATE INDEX IF NOT EXISTS idx_workspaces_cloud_id ON workspaces(cloud_id);
