-- Your SQL goes here
CREATE TABLE organisations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    tel TEXT,
    email TEXT,
    address TEXT,
    description TEXT,
    location_country_id UUID REFERENCES countries(id),
    organisation_type_id UUID REFERENCES organisation_types(id),
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);
