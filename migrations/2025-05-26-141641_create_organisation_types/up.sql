-- Your SQL goes here
CREATE TABLE organisation_types (
   id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
   type TEXT NOT NULL,
   color TEXT
);
