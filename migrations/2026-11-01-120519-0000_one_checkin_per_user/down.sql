-- The older duplicate rows removed by the up migration are not restored.
DROP INDEX IF EXISTS org_checkins_org_user_key;
