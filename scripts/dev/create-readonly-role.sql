-- Read-only role for local inspection tools (Postgres MCP, ad-hoc agents).
-- LOCAL DEVELOPMENT ONLY. Idempotent: re-run it after migrations that add
-- tables with secrets. Run through scripts/dev/setup-readonly-db.sh, or:
--
--   docker exec -i landly_postgres psql -U landly-user -d landly \
--       -v ro_password='…' < scripts/dev/create-readonly-role.sql
--
-- Must run as the role that owns the tables / runs migrations (the compose
-- POSTGRES_USER), so DEFAULT PRIVILEGES cover tables created later.
\set ON_ERROR_STOP on
SET client_min_messages = warning;

SELECT 'CREATE ROLE landly_readonly LOGIN'
WHERE NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'landly_readonly') \gexec

ALTER ROLE landly_readonly WITH LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE PASSWORD :'ro_password';
-- Belt and braces: even if a tool forgets to open a read-only transaction,
-- the session default refuses writes.
ALTER ROLE landly_readonly SET default_transaction_read_only = on;
ALTER ROLE landly_readonly SET statement_timeout = '30s';

SELECT format('GRANT CONNECT ON DATABASE %I TO landly_readonly', current_database()) \gexec
GRANT USAGE ON SCHEMA public TO landly_readonly;
GRANT SELECT ON ALL TABLES IN SCHEMA public TO landly_readonly;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT SELECT ON TABLES TO landly_readonly;
-- Index/health tools read the statistics views.
GRANT pg_read_all_stats TO landly_readonly;

-- Secrets stay invisible: password hashes and person claim tokens (the token
-- IS the credential for /api/person/claim/{token}). Column-level grants for
-- everything else in those tables.
REVOKE SELECT ON users, person_claim_tokens FROM landly_readonly;
SELECT format(
           'GRANT SELECT (%s) ON %I TO landly_readonly',
           string_agg(quote_ident(column_name), ', ' ORDER BY ordinal_position),
           table_name)
FROM information_schema.columns
WHERE table_schema = 'public'
  AND ((table_name = 'users' AND column_name <> 'password_hash')
    OR (table_name = 'person_claim_tokens' AND column_name <> 'token'))
GROUP BY table_name \gexec
