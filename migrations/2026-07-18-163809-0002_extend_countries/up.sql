-- Country v2 fields (design: country-full.jsx)
ALTER TABLE countries
    ADD COLUMN currency TEXT,
    ADD COLUMN phone_code TEXT,
    ADD COLUMN top_cities JSONB;
