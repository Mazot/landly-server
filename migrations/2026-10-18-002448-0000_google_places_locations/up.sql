-- Google Maps Platform as the map provider (see docs/operations.md →
-- "Google Places"). Google's terms allow storing only the place ID forever;
-- coordinates from Places may be cached and must be refreshed (< 30 days),
-- everything else (rating, hours, photos) is fetched live by the client.

-- 1. Never store Google content we may not keep: the imported rating goes.
ALTER TABLE organisations DROP COLUMN google_rating;

-- 2. Where a point came from, and when it was last fetched from Google.
--    google    — coordinates of `google_place_id`, refreshed by the server
--    user_pin  — placed by the submitter (our own data, never refreshed)
--    moderator — moved by a moderator (our own data, never refreshed)
ALTER TABLE organisations
    ADD COLUMN location_source TEXT,
    ADD COLUMN location_refreshed_at TIMESTAMP;

UPDATE organisations
SET location_source = CASE WHEN google_place_id IS NOT NULL THEN 'google' ELSE 'user_pin' END
WHERE latitude IS NOT NULL AND longitude IS NOT NULL;

ALTER TABLE organisations
    ADD CONSTRAINT organisations_location_source_check
        CHECK (location_source IS NULL OR location_source IN ('google', 'user_pin', 'moderator'));

-- 3. A point is a pair: a lone latitude or longitude can never be shown on
--    the map (search needs both), so such leftovers are cleared first.
UPDATE organisations
SET latitude = NULL, longitude = NULL, location_source = NULL
WHERE (latitude IS NULL) <> (longitude IS NULL);

ALTER TABLE organisations
    ADD CONSTRAINT organisations_lat_lng_pair_check
        CHECK ((latitude IS NULL) = (longitude IS NULL));

-- 4. One Google place = one organisation (exact duplicate detection).
--    If this fails with "could not create unique index", duplicates exist:
--      SELECT google_place_id, array_agg(id) FROM organisations
--      WHERE google_place_id IS NOT NULL GROUP BY 1 HAVING count(*) > 1;
--    merge or clear them, then re-run.
CREATE UNIQUE INDEX organisations_google_place_id_key
    ON organisations (google_place_id) WHERE google_place_id IS NOT NULL;
