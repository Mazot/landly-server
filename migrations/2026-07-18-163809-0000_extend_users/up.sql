-- Profile v2 fields (design: account.jsx / signup-corridor.jsx)
ALTER TABLE users
    ADD COLUMN name TEXT,
    ADD COLUMN bio TEXT,
    ADD COLUMN city TEXT,
    ADD COLUMN home_country_id UUID REFERENCES countries(id) ON DELETE SET NULL,
    ADD COLUMN avatar_color TEXT,
    ADD COLUMN locale TEXT NOT NULL DEFAULT 'en',
    ADD COLUMN here_as TEXT,
    ADD COLUMN role TEXT NOT NULL DEFAULT 'user',
    ADD COLUMN notification_settings JSONB;

ALTER TABLE users
    ADD CONSTRAINT users_locale_check CHECK (locale IN ('en', 'ru', 'uk')),
    ADD CONSTRAINT users_here_as_check CHECK (here_as IS NULL OR here_as IN ('newcomer', 'helping', 'exploring')),
    ADD CONSTRAINT users_role_check CHECK (role IN ('user', 'moderator', 'admin'));
