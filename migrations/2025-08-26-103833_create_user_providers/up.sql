-- Your SQL goes here
CREATE TABLE user_providers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider VARCHAR(128) NOT NULL,
    provider_user_id VARCHAR(128) NOT NULL,
    email VARCHAR(255),
    created_at TIMESTAMP DEFAULT NOW(),
    UNIQUE(provider, provider_user_id)
);
