-- Your SQL goes here
CREATE TABLE countries_to_languages (
    country_id UUID REFERENCES countries(id),
    language_id UUID REFERENCES languages(id),
    PRIMARY KEY (country_id, language_id)
);
