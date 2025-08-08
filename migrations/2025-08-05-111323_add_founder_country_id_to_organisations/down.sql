-- This file should undo anything in `up.sql`
ALTER TABLE organisations
DROP COLUMN founder_country_id;
