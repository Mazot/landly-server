-- A check-in was one row per tap, so one user could inflate "people came" and
-- "still active" for a place without limit (audit M4). From now on there is
-- one row per (organisation, user): checking in again updates it.

-- Keep the user's latest tip on the surviving (newest) row...
UPDATE org_checkins c
SET tip = (
    SELECT o.tip FROM org_checkins o
    WHERE o.organisation_id = c.organisation_id
      AND o.user_id = c.user_id
      AND o.tip IS NOT NULL
    ORDER BY o.created_at DESC, o.id DESC
    LIMIT 1
)
WHERE c.tip IS NULL;

-- ...then drop the older rows of the same (organisation, user).
DELETE FROM org_checkins c
USING org_checkins newer
WHERE c.organisation_id = newer.organisation_id
  AND c.user_id = newer.user_id
  AND (c.created_at, c.id) < (newer.created_at, newer.id);

CREATE UNIQUE INDEX org_checkins_org_user_key ON org_checkins (organisation_id, user_id);
