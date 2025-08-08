-- Your SQL goes here
ALTER TABLE organisations
ADD COLUMN founder_country_id UUID REFERENCES countries(id);
