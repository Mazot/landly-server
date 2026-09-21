-- Seed test data for organisations and country connections
-- Uses only countries that exist in the DB

-- Country ID reference:
-- Germany:        8c015571-4f25-4bc6-b939-c37fbd3ed764
-- Italy:          f790947b-1621-4e5d-93fb-d7f2a622dc56
-- Japan:          5f87d98b-c0de-4be1-a109-13542a5e3c84
-- Poland:         7b399d0a-caf9-44b0-8ac5-d054fbacbb4e
-- Spain:          567f27e2-8885-46dd-acb2-dd53d11c890e
-- Turkey:         9ce4f31b-7dd9-4b52-88b7-e3595096444a
-- Ukraine:        2ba2c562-0fe4-483f-b1b6-f17bd217de0c
-- United Kingdom: fd172532-f9d4-43f7-9bb8-55a8a25b4fb9
-- United States:  f6ec0017-b817-45d1-aa2d-910053a14fe5

BEGIN;

-- =============================================
-- Countries referenced below by fixed IDs. Inserted idempotently so this
-- file runs on a freshly migrated database (country_loader assigns random
-- UUIDs, so these fixed test IDs must be created here).
-- =============================================
INSERT INTO countries (id, name) VALUES
  ('8c015571-4f25-4bc6-b939-c37fbd3ed764', 'Germany'),
  ('f790947b-1621-4e5d-93fb-d7f2a622dc56', 'Italy'),
  ('5f87d98b-c0de-4be1-a109-13542a5e3c84', 'Japan'),
  ('7b399d0a-caf9-44b0-8ac5-d054fbacbb4e', 'Poland'),
  ('567f27e2-8885-46dd-acb2-dd53d11c890e', 'Spain'),
  ('9ce4f31b-7dd9-4b52-88b7-e3595096444a', 'Turkey'),
  ('2ba2c562-0fe4-483f-b1b6-f17bd217de0c', 'Ukraine'),
  ('fd172532-f9d4-43f7-9bb8-55a8a25b4fb9', 'United Kingdom'),
  ('f6ec0017-b817-45d1-aa2d-910053a14fe5', 'United States')
ON CONFLICT (id) DO NOTHING;

-- =============================================
-- Organisation types referenced below by fixed IDs.
-- Nothing else creates these rows (migrations seed the canonical slugged
-- types with random UUIDs), so insert them idempotently to keep this file
-- runnable on a freshly migrated database.
-- =============================================
INSERT INTO organisation_types (id, type, color, title) VALUES
  ('a1000000-0000-0000-0000-000000000001', 'embassy', '#4A6D8C', 'Embassy'),
  ('a1000000-0000-0000-0000-000000000002', 'consulate', '#4A6D8C', 'Consulate'),
  ('a1000000-0000-0000-0000-000000000003', 'business', '#B8814A', 'Business'),
  ('a1000000-0000-0000-0000-000000000004', 'ngo', '#4F7F5C', 'NGO'),
  ('a1000000-0000-0000-0000-000000000005', 'cultural_center', '#8B5A8C', 'Cultural Center')
ON CONFLICT (id) DO NOTHING;


-- =============================================
-- Organisations
-- =============================================
INSERT INTO organisations (id, name, tel, email, address, description, location_country_id, organisation_type_id, latitude, longitude, founder_country_id) VALUES
  -- Embassies
  ('b1000000-0000-0000-0000-000000000001', 'Embassy of Ukraine in Germany', '+49 30 288 87 128', 'emb_de@mfa.gov.ua', 'Albrechtstraße 26, 10117 Berlin', 'Official diplomatic mission of Ukraine in Germany', '8c015571-4f25-4bc6-b939-c37fbd3ed764', 'a1000000-0000-0000-0000-000000000001', 52.52437587, 13.38333580, '2ba2c562-0fe4-483f-b1b6-f17bd217de0c'),
  ('b1000000-0000-0000-0000-000000000003', 'Embassy of Germany in Ukraine', '+380 44 247 68 00', 'info@kiew.diplo.de', 'Bohdana Khmelnytskoho St, 25, Kyiv', 'Official diplomatic mission of Germany in Ukraine', '2ba2c562-0fe4-483f-b1b6-f17bd217de0c', 'a1000000-0000-0000-0000-000000000001', 50.44128300, 30.52291700, '8c015571-4f25-4bc6-b939-c37fbd3ed764'),
  ('b1000000-0000-0000-0000-000000000004', 'Embassy of Ukraine in Italy', '+39 06 841 26 30', 'emb_it@mfa.gov.ua', 'Via Guido dArezzo 9, 00198 Roma', 'Official diplomatic mission of Ukraine in Italy', 'f790947b-1621-4e5d-93fb-d7f2a622dc56', 'a1000000-0000-0000-0000-000000000001', 41.92186000, 12.49545600, '2ba2c562-0fe4-483f-b1b6-f17bd217de0c'),
  ('b1000000-0000-0000-0000-000000000005', 'Embassy of Ukraine in Poland', '+48 22 629 34 46', 'emb_pl@mfa.gov.ua', 'Al. Szucha 7, 00-580 Warsaw', 'Official diplomatic mission of Ukraine in Poland', '7b399d0a-caf9-44b0-8ac5-d054fbacbb4e', 'a1000000-0000-0000-0000-000000000001', 52.22082500, 21.02271100, '2ba2c562-0fe4-483f-b1b6-f17bd217de0c'),
  ('b1000000-0000-0000-0000-000000000007', 'Embassy of Ukraine in USA', '+1 202 349 2963', 'emb_us@mfa.gov.ua', '3350 M St NW, Washington, DC 20007', 'Official diplomatic mission of Ukraine in the United States', 'f6ec0017-b817-45d1-aa2d-910053a14fe5', 'a1000000-0000-0000-0000-000000000001', 38.90502000, -77.06370000, '2ba2c562-0fe4-483f-b1b6-f17bd217de0c'),
  ('b1000000-0000-0000-0000-000000000008', 'Embassy of Ukraine in United Kingdom', '+44 20 7727 6312', 'emb_gb@mfa.gov.ua', '60 Holland Park, London W11 3SJ', 'Official diplomatic mission of Ukraine in the UK', 'fd172532-f9d4-43f7-9bb8-55a8a25b4fb9', 'a1000000-0000-0000-0000-000000000001', 51.50525900, -0.20326300, '2ba2c562-0fe4-483f-b1b6-f17bd217de0c'),
  ('b1000000-0000-0000-0000-000000000009', 'Embassy of Japan in Germany', '+49 30 210 94 0', 'info@bo.mofa.go.jp', 'Hiroshimastraße 6, 10785 Berlin', 'Official diplomatic mission of Japan in Germany', '8c015571-4f25-4bc6-b939-c37fbd3ed764', 'a1000000-0000-0000-0000-000000000001', 52.50823700, 13.36052400, '5f87d98b-c0de-4be1-a109-13542a5e3c84'),
  ('b1000000-0000-0000-0000-000000000016', 'Embassy of Spain in Germany', '+49 30 254 007 0', 'emb.berlin@maec.es', 'Lichtensteinallee 1, 10787 Berlin', 'Official diplomatic mission of Spain in Germany', '8c015571-4f25-4bc6-b939-c37fbd3ed764', 'a1000000-0000-0000-0000-000000000001', 52.51300000, 13.34800000, '567f27e2-8885-46dd-acb2-dd53d11c890e'),
  ('b1000000-0000-0000-0000-000000000017', 'Embassy of Italy in United Kingdom', '+44 20 7312 2200', 'ambasciata.londra@esteri.it', '14 Three Kings Yard, London W1K 4EH', 'Official diplomatic mission of Italy in the UK', 'fd172532-f9d4-43f7-9bb8-55a8a25b4fb9', 'a1000000-0000-0000-0000-000000000001', 51.51279000, -0.14907000, 'f790947b-1621-4e5d-93fb-d7f2a622dc56'),
  ('b1000000-0000-0000-0000-000000000021', 'Embassy of Turkey in Ukraine', '+380 44 281 07 50', 'embassy.kyiv@mfa.gov.tr', 'Arsenalna St, 18, Kyiv', 'Official diplomatic mission of Turkey in Ukraine', '2ba2c562-0fe4-483f-b1b6-f17bd217de0c', 'a1000000-0000-0000-0000-000000000001', 50.44320000, 30.54430000, '9ce4f31b-7dd9-4b52-88b7-e3595096444a'),

  -- Consulates
  ('b1000000-0000-0000-0000-000000000002', 'Consulate of Ukraine in Hamburg', '+49 40 220 99 59', 'gc_deh@mfa.gov.ua', 'Mundsburger Damm 1, 22087 Hamburg', 'Consular services for Ukrainian citizens in Hamburg region', '8c015571-4f25-4bc6-b939-c37fbd3ed764', 'a1000000-0000-0000-0000-000000000002', 53.57072800, 10.02249600, '2ba2c562-0fe4-483f-b1b6-f17bd217de0c'),
  ('b1000000-0000-0000-0000-000000000006', 'Consulate of Poland in Lviv', '+380 32 297 08 61', 'lwow.kg.sekretariat@msz.gov.pl', 'Ivana Franka St, 110, Lviv', 'Polish consular services in western Ukraine', '2ba2c562-0fe4-483f-b1b6-f17bd217de0c', 'a1000000-0000-0000-0000-000000000002', 49.83989200, 24.02967200, '7b399d0a-caf9-44b0-8ac5-d054fbacbb4e'),
  ('b1000000-0000-0000-0000-000000000010', 'Consulate of Turkey in Frankfurt', '+49 69 233 081', 'konsulat.frankfurt@mfa.gov.tr', 'Zeppelinallee 25, 60325 Frankfurt', 'Turkish consular services in Frankfurt area', '8c015571-4f25-4bc6-b939-c37fbd3ed764', 'a1000000-0000-0000-0000-000000000002', 50.11552100, 8.66090700, '9ce4f31b-7dd9-4b52-88b7-e3595096444a'),
  ('b1000000-0000-0000-0000-000000000019', 'Consulate of Ukraine in Istanbul', '+90 212 252 12 19', 'gc_tri@mfa.gov.ua', 'Sahkulu Mah., Galip Dede Cad. 37, Beyoglu, Istanbul', 'Consular services for Ukrainian citizens in Turkey', '9ce4f31b-7dd9-4b52-88b7-e3595096444a', 'a1000000-0000-0000-0000-000000000002', 41.02585000, 28.97459700, '2ba2c562-0fe4-483f-b1b6-f17bd217de0c'),
  ('b1000000-0000-0000-0000-000000000022', 'Consulate of Ukraine in Milan', '+39 02 295 14 470', 'gc_mil@mfa.gov.ua', 'Via Ludovico di Breme 11, 20156 Milano', 'Ukrainian consular services in northern Italy', 'f790947b-1621-4e5d-93fb-d7f2a622dc56', 'a1000000-0000-0000-0000-000000000002', 45.49474200, 9.15414900, '2ba2c562-0fe4-483f-b1b6-f17bd217de0c'),

  -- Businesses
  ('b1000000-0000-0000-0000-000000000011', 'Berlin Tech Hub GmbH', '+49 30 123 456 78', 'info@berlintechhub.de', 'Friedrichstrasse 68, 10117 Berlin', 'International tech coworking and startup accelerator', '8c015571-4f25-4bc6-b939-c37fbd3ed764', 'a1000000-0000-0000-0000-000000000003', 52.52000000, 13.38870000, '8c015571-4f25-4bc6-b939-c37fbd3ed764'),
  ('b1000000-0000-0000-0000-000000000012', 'EastWest Trade sp. z o.o.', '+48 22 555 12 34', 'contact@eastwesttrade.pl', 'ul. Marszalkowska 84, 00-514 Warsaw', 'Import/export services specializing in Eastern European trade', '7b399d0a-caf9-44b0-8ac5-d054fbacbb4e', 'a1000000-0000-0000-0000-000000000003', 52.22967800, 21.01223100, '7b399d0a-caf9-44b0-8ac5-d054fbacbb4e'),
  ('b1000000-0000-0000-0000-000000000018', 'Sakura IT Consulting KK', '+81 3 1234 5678', 'info@sakurait.jp', '2-3-1 Marunouchi, Chiyoda, Tokyo', 'IT consulting and software development services', '5f87d98b-c0de-4be1-a109-13542a5e3c84', 'a1000000-0000-0000-0000-000000000003', 35.68124400, NULL, '5f87d98b-c0de-4be1-a109-13542a5e3c84'),
  ('b1000000-0000-0000-0000-000000000020', 'LinguaBridge Translation Services', '+48 22 876 54 32', 'office@linguabridge.pl', 'ul. Nowy Swiat 33, 00-029 Warsaw', 'Professional translation and interpretation services', '7b399d0a-caf9-44b0-8ac5-d054fbacbb4e', 'a1000000-0000-0000-0000-000000000003', 52.23108500, 21.01819200, 'f6ec0017-b817-45d1-aa2d-910053a14fe5'),

  -- NGOs
  ('b1000000-0000-0000-0000-000000000013', 'HelpUA Foundation', '+380 44 333 22 11', 'info@helpua.org', 'Khreshchatyk St, 22, Kyiv', 'Humanitarian aid and support for displaced persons', '2ba2c562-0fe4-483f-b1b6-f17bd217de0c', 'a1000000-0000-0000-0000-000000000004', 50.44941000, 30.52468100, '2ba2c562-0fe4-483f-b1b6-f17bd217de0c'),
  ('b1000000-0000-0000-0000-000000000014', 'UK-Ukraine Cultural Exchange', '+44 20 8123 4567', 'hello@ukukraine.org.uk', '15 Kensington High St, London W8 5NP', 'Promoting cultural exchange between UK and Ukraine', 'fd172532-f9d4-43f7-9bb8-55a8a25b4fb9', 'a1000000-0000-0000-0000-000000000004', 51.50190200, -0.18764800, 'fd172532-f9d4-43f7-9bb8-55a8a25b4fb9'),

  -- Cultural Centers
  ('b1000000-0000-0000-0000-000000000015', 'Ukrainian Cultural Center Rome', '+39 06 442 51 00', 'culture@ukrainerome.it', 'Via della Conciliazione 10, 00193 Roma', 'Ukrainian cultural center promoting art, music and language', 'f790947b-1621-4e5d-93fb-d7f2a622dc56', 'a1000000-0000-0000-0000-000000000005', 41.90216600, 12.46015300, '2ba2c562-0fe4-483f-b1b6-f17bd217de0c');

-- =============================================
-- Country Connections
-- =============================================
INSERT INTO countries_connections (id, embassy_org_id, consulate_org_id, common_info, location_country_id) VALUES
  -- Ukraine diplomatic in Germany (embassy + consulate)
  ('c1000000-0000-0000-0000-000000000001', 'b1000000-0000-0000-0000-000000000001', 'b1000000-0000-0000-0000-000000000002', 'Ukraine diplomatic representation in Germany. Embassy in Berlin handles political and economic affairs. Consulate in Hamburg provides consular services.', '8c015571-4f25-4bc6-b939-c37fbd3ed764'),
  -- Germany diplomatic in Ukraine
  ('c1000000-0000-0000-0000-000000000002', 'b1000000-0000-0000-0000-000000000003', NULL, 'German diplomatic representation in Ukraine. Embassy in Kyiv handles visa and consular affairs.', '2ba2c562-0fe4-483f-b1b6-f17bd217de0c'),
  -- Ukraine diplomatic in Italy (embassy + consulate)
  ('c1000000-0000-0000-0000-000000000003', 'b1000000-0000-0000-0000-000000000004', 'b1000000-0000-0000-0000-000000000022', 'Ukrainian diplomatic presence in Italy. Embassy in Rome, Consulate in Milan.', 'f790947b-1621-4e5d-93fb-d7f2a622dc56'),
  -- Ukraine diplomatic in Poland
  ('c1000000-0000-0000-0000-000000000004', 'b1000000-0000-0000-0000-000000000005', NULL, 'Ukraine diplomatic representation in Poland. Embassy in Warsaw.', '7b399d0a-caf9-44b0-8ac5-d054fbacbb4e'),
  -- Poland diplomatic in Ukraine (consulate only)
  ('c1000000-0000-0000-0000-000000000005', NULL, 'b1000000-0000-0000-0000-000000000006', 'Polish consulate in Lviv serves western Ukraine. Visa and citizen services.', '2ba2c562-0fe4-483f-b1b6-f17bd217de0c'),
  -- Ukraine diplomatic in USA
  ('c1000000-0000-0000-0000-000000000006', 'b1000000-0000-0000-0000-000000000007', NULL, 'Ukrainian diplomatic mission in the United States. Embassy in Washington DC.', 'f6ec0017-b817-45d1-aa2d-910053a14fe5'),
  -- Ukraine diplomatic in UK
  ('c1000000-0000-0000-0000-000000000007', 'b1000000-0000-0000-0000-000000000008', NULL, 'Ukrainian diplomatic mission in the United Kingdom. Embassy in London.', 'fd172532-f9d4-43f7-9bb8-55a8a25b4fb9'),
  -- Japan diplomatic in Germany
  ('c1000000-0000-0000-0000-000000000008', 'b1000000-0000-0000-0000-000000000009', NULL, 'Japanese diplomatic mission in Germany. Embassy in Berlin.', '8c015571-4f25-4bc6-b939-c37fbd3ed764'),
  -- Turkey diplomatic in Germany (consulate only)
  ('c1000000-0000-0000-0000-000000000009', NULL, 'b1000000-0000-0000-0000-000000000010', 'Turkish consular representation in Germany. Consulate in Frankfurt.', '8c015571-4f25-4bc6-b939-c37fbd3ed764'),
  -- Ukraine diplomatic in Turkey (consulate only)
  ('c1000000-0000-0000-0000-000000000010', NULL, 'b1000000-0000-0000-0000-000000000019', 'Ukrainian consular services in Turkey. Consulate General in Istanbul.', '9ce4f31b-7dd9-4b52-88b7-e3595096444a'),
  -- Spain diplomatic in Germany
  ('c1000000-0000-0000-0000-000000000011', 'b1000000-0000-0000-0000-000000000016', NULL, 'Spanish diplomatic mission in Germany. Embassy in Berlin.', '8c015571-4f25-4bc6-b939-c37fbd3ed764'),
  -- Italy diplomatic in UK
  ('c1000000-0000-0000-0000-000000000012', 'b1000000-0000-0000-0000-000000000017', NULL, 'Italian diplomatic mission in the United Kingdom. Embassy in London.', 'fd172532-f9d4-43f7-9bb8-55a8a25b4fb9'),
  -- Turkey diplomatic in Ukraine
  ('c1000000-0000-0000-0000-000000000013', 'b1000000-0000-0000-0000-000000000021', NULL, 'Turkish diplomatic mission in Ukraine. Embassy in Kyiv.', '2ba2c562-0fe4-483f-b1b6-f17bd217de0c');

COMMIT;
