ALTER TABLE users
    DROP CONSTRAINT users_role_check,
    DROP CONSTRAINT users_here_as_check,
    DROP CONSTRAINT users_locale_check;

ALTER TABLE users
    DROP COLUMN notification_settings,
    DROP COLUMN role,
    DROP COLUMN here_as,
    DROP COLUMN locale,
    DROP COLUMN avatar_color,
    DROP COLUMN home_country_id,
    DROP COLUMN city,
    DROP COLUMN bio,
    DROP COLUMN name;
