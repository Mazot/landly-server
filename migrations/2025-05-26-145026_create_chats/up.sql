-- Your SQL goes here
CREATE TABLE chats (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  app TEXT,
  origin_country_connection_id UUID REFERENCES countries_connections(id),
  link TEXT,
  info TEXT
);
