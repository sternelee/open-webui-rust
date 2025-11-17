-- OAuth Session Table (matching Python backend schema)
-- Drop existing incompatible table if it exists
DROP TABLE IF EXISTS oauth_session CASCADE;

-- Create oauth_session table matching Python backend schema
CREATE TABLE IF NOT EXISTS oauth_session (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    provider TEXT NOT NULL,
    token TEXT NOT NULL,  -- Encrypted JSON containing access_token, refresh_token, id_token, etc.
    expires_at BIGINT NOT NULL,  -- Unix timestamp
    created_at BIGINT NOT NULL,  -- Unix timestamp
    updated_at BIGINT NOT NULL,  -- Unix timestamp
    FOREIGN KEY (user_id) REFERENCES "user"(id) ON DELETE CASCADE
);

-- Create indexes for better performance
CREATE INDEX IF NOT EXISTS idx_oauth_session_user_id ON oauth_session(user_id);
CREATE INDEX IF NOT EXISTS idx_oauth_session_expires_at ON oauth_session(expires_at);
CREATE INDEX IF NOT EXISTS idx_oauth_session_user_provider ON oauth_session(user_id, provider);

-- Comment on the table
COMMENT ON TABLE oauth_session IS 'Stores encrypted OAuth session tokens for users';
COMMENT ON COLUMN oauth_session.token IS 'Encrypted JSON containing access_token, refresh_token, id_token, expires_at, etc.';

