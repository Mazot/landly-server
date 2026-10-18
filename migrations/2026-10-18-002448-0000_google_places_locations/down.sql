DROP INDEX IF EXISTS organisations_google_place_id_key;

ALTER TABLE organisations
    DROP CONSTRAINT IF EXISTS organisations_lat_lng_pair_check,
    DROP CONSTRAINT IF EXISTS organisations_location_source_check,
    DROP COLUMN location_refreshed_at,
    DROP COLUMN location_source;

-- The dropped ratings are not restored (they were Google content we may
-- not keep); the column comes back empty.
ALTER TABLE organisations ADD COLUMN google_rating DOUBLE PRECISION;
