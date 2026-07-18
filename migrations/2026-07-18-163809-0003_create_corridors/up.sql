-- User corridors: from-country -> to-country pairs the map opens to
-- (design: signup-corridor.jsx, map-corridor.jsx, account.jsx)
CREATE TABLE corridors (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    from_country_id UUID NOT NULL REFERENCES countries(id) ON DELETE CASCADE,
    to_country_id UUID NOT NULL REFERENCES countries(id) ON DELETE CASCADE,
    is_default BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    CONSTRAINT corridors_distinct_countries CHECK (from_country_id <> to_country_id),
    CONSTRAINT corridors_unique_per_user UNIQUE (user_id, from_country_id, to_country_id)
);

CREATE INDEX idx_corridors_user_id ON corridors(user_id);
-- At most one default corridor per user
CREATE UNIQUE INDEX idx_corridors_one_default_per_user ON corridors(user_id) WHERE is_default = TRUE;
