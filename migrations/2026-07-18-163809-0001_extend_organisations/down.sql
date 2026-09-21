DROP INDEX idx_organisation_types_slug;
ALTER TABLE organisation_types DROP COLUMN slug;

DROP INDEX idx_organisations_status;
DROP INDEX idx_organisations_lat_lng;

ALTER TABLE organisations
    DROP CONSTRAINT organisations_cost_check,
    DROP CONSTRAINT organisations_added_by_check,
    DROP CONSTRAINT organisations_status_check;

ALTER TABLE organisations
    DROP COLUMN reviews_count,
    DROP COLUMN rating_avg,
    DROP COLUMN visits_count,
    DROP COLUMN google_rating,
    DROP COLUMN google_place_id,
    DROP COLUMN cost,
    DROP COLUMN timezone,
    DROP COLUMN opening_hours,
    DROP COLUMN languages,
    DROP COLUMN services,
    DROP COLUMN whatsapp,
    DROP COLUMN telegram,
    DROP COLUMN website,
    DROP COLUMN city,
    DROP COLUMN added_by,
    DROP COLUMN moderation_note,
    DROP COLUMN status,
    DROP COLUMN verified,
    DROP COLUMN created_by;
