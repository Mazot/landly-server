-- Your SQL goes here
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

CREATE TABLE countries (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    geo_json JSONB,
    flag TEXT,
    capital_city TEXT,
    description TEXT
);
