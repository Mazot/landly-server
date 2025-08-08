-- Your SQL goes here
CREATE TABLE users_to_languages (
    user_id UUID REFERENCES users(id),
    language_id UUID REFERENCES languages(id),
    PRIMARY KEY (user_id, language_id)
);
