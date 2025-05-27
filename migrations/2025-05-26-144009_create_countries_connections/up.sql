-- Your SQL goes here
CREATE TABLE countries_connections (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    embassy_org_id UUID REFERENCES organisations(id),
    consulate_org_id UUID REFERENCES organisations(id),
    common_info TEXT,
    location_country_id UUID REFERENCES countries(id)
);
