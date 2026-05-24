-- Additional test data: more org types, more countries, more connections
-- Adds to existing seed data

-- Country IDs:
-- (existing)
-- Germany:        8c015571-4f25-4bc6-b939-c37fbd3ed764
-- Italy:          f790947b-1621-4e5d-93fb-d7f2a622dc56
-- Japan:          5f87d98b-c0de-4be1-a109-13542a5e3c84
-- Poland:         7b399d0a-caf9-44b0-8ac5-d054fbacbb4e
-- Spain:          567f27e2-8885-46dd-acb2-dd53d11c890e
-- Turkey:         9ce4f31b-7dd9-4b52-88b7-e3595096444a
-- Ukraine:        2ba2c562-0fe4-483f-b1b6-f17bd217de0c
-- United Kingdom: fd172532-f9d4-43f7-9bb8-55a8a25b4fb9
-- United States:  f6ec0017-b817-45d1-aa2d-910053a14fe5
-- (new)
-- Argentina:      39441314-5282-4502-bad5-0ca17923287a
-- Australia:      ec9f55d7-f2fe-4fdb-a937-1509e3c24e0b
-- Brazil:         17fca769-c38e-4a5b-92af-a1ec48576957
-- Canada:         955d8e2a-d3d5-48b3-b2c0-0b02d5fe49d0
-- China:          808ac4a6-6994-4527-8935-0b2557d96837
-- Egypt:          7ae8acad-671c-4b31-af7a-2526826af590
-- Greece:         80246ecb-b469-48d0-b33d-64451d81564e
-- India:          726c7255-42d8-40a5-891f-ea1263d73bc9
-- Israel:         6de9c02b-c30f-4c74-8217-d3ccdf876542
-- Kenya:          4250abff-f03a-468b-b1d7-3bb69cc9e2dd
-- Mexico:         ea693ddc-74a0-4680-bf62-0823dbf42ce9
-- Morocco:        a4b1ae14-829d-44b2-9202-aaa11fb956fc
-- Nigeria:        fbf576bf-a760-47fb-bae7-751470acc607
-- Portugal:       da7b943a-13d3-4365-bb63-0db1c0d0764f
-- Romania:        46486ec6-1126-41a7-9e29-e7bb79cd44d6
-- Saudi Arabia:   9d745ed6-b924-4ad3-a20a-eb2949c9b2a4
-- Singapore:      bdd7fb80-d75e-406e-bd29-7a36a66f7383
-- South Africa:   e0d4a87e-a501-4011-a113-33708f87c31e
-- South Korea:    07be8c59-3289-4ca4-bbf5-f07c6de276ef
-- Sweden:         6d399ecf-127f-4bc3-9c05-7799ac9d5d4f
-- Switzerland:    37608878-0cb1-4428-84d5-49a0dbabce71
-- Thailand:       6568352c-2f34-450f-9b3f-7a91229a11c4
-- UAE:            3292614f-0e99-42f8-b188-0ece2ea32f5a
-- Vietnam:        9a5979ef-1653-4470-b2c8-dadf0fcb7381

-- Org type IDs:
-- Embassy:         a1000000-0000-0000-0000-000000000001
-- Consulate:       a1000000-0000-0000-0000-000000000002
-- Business:        a1000000-0000-0000-0000-000000000003
-- NGO:             a1000000-0000-0000-0000-000000000004
-- Cultural Center: a1000000-0000-0000-0000-000000000005
-- (new types)
-- Trade Office:    a1000000-0000-0000-0000-000000000006
-- Research Inst:   a1000000-0000-0000-0000-000000000007
-- School:          a1000000-0000-0000-0000-000000000008

BEGIN;

-- =============================================
-- New Organisation Types
-- =============================================
INSERT INTO organisation_types (id, type, color, title) VALUES
  ('a1000000-0000-0000-0000-000000000006', 'trade_office', '#0891B2', 'Trade Office'),
  ('a1000000-0000-0000-0000-000000000007', 'research_institute', '#6366F1', 'Research Institute'),
  ('a1000000-0000-0000-0000-000000000008', 'school', '#F59E0B', 'International School');

-- =============================================
-- Organisations (40 new entries)
-- =============================================
INSERT INTO organisations (id, name, tel, email, address, description, location_country_id, organisation_type_id, latitude, longitude, founder_country_id) VALUES

  -- === EMBASSIES ===
  -- Brazil Embassy in Germany
  ('b2000000-0000-0000-0000-000000000001', 'Embassy of Brazil in Germany', '+49 30 72628 0', 'brasemb.berlim@itamaraty.gov.br', 'Wallstrasse 57, 10179 Berlin', 'Diplomatic mission of Brazil in Germany', '8c015571-4f25-4bc6-b939-c37fbd3ed764', 'a1000000-0000-0000-0000-000000000001', 52.51220000, 13.41110000, '17fca769-c38e-4a5b-92af-a1ec48576957'),
  -- India Embassy in USA
  ('b2000000-0000-0000-0000-000000000002', 'Embassy of India in USA', '+1 202 939 7000', 'hoc.washington@mea.gov.in', '2107 Massachusetts Ave NW, Washington DC 20008', 'Diplomatic mission of India in the United States', 'f6ec0017-b817-45d1-aa2d-910053a14fe5', 'a1000000-0000-0000-0000-000000000001', 38.91340000, -77.04860000, '726c7255-42d8-40a5-891f-ea1263d73bc9'),
  -- China Embassy in UK
  ('b2000000-0000-0000-0000-000000000003', 'Embassy of China in United Kingdom', '+44 20 7299 4049', 'chinaemb_gb@mfa.gov.cn', '49-51 Portland Place, London W1B 1JL', 'Diplomatic mission of China in the UK', 'fd172532-f9d4-43f7-9bb8-55a8a25b4fb9', 'a1000000-0000-0000-0000-000000000001', 51.52080000, -0.14580000, '808ac4a6-6994-4527-8935-0b2557d96837'),
  -- Australia Embassy in Japan
  ('b2000000-0000-0000-0000-000000000004', 'Embassy of Australia in Japan', '+81 3 5232 4111', 'austemb.tokyo@dfat.gov.au', '2-1-14 Mita, Minato-ku, Tokyo', 'Diplomatic mission of Australia in Japan', '5f87d98b-c0de-4be1-a109-13542a5e3c84', 'a1000000-0000-0000-0000-000000000001', 35.65150000, NULL, 'ec9f55d7-f2fe-4fdb-a937-1509e3c24e0b'),
  -- South Korea Embassy in Germany
  ('b2000000-0000-0000-0000-000000000005', 'Embassy of South Korea in Germany', '+49 30 260 650', 'koremb-ge@mofa.go.kr', 'Stuflerstrasse 8-10, 10787 Berlin', 'Diplomatic mission of South Korea in Germany', '8c015571-4f25-4bc6-b939-c37fbd3ed764', 'a1000000-0000-0000-0000-000000000001', 52.50680000, 13.34950000, '07be8c59-3289-4ca4-bbf5-f07c6de276ef'),
  -- Canada Embassy in Italy
  ('b2000000-0000-0000-0000-000000000006', 'Embassy of Canada in Italy', '+39 06 854 441', 'rome@international.gc.ca', 'Via Zara 30, 00198 Roma', 'Diplomatic mission of Canada in Italy', 'f790947b-1621-4e5d-93fb-d7f2a622dc56', 'a1000000-0000-0000-0000-000000000001', 41.92410000, 12.49930000, '955d8e2a-d3d5-48b3-b2c0-0b02d5fe49d0'),
  -- Mexico Embassy in Spain
  ('b2000000-0000-0000-0000-000000000007', 'Embassy of Mexico in Spain', '+34 91 369 2814', 'embamex.espana@sre.gob.mx', 'Carrera de San Jeronimo 46, 28014 Madrid', 'Diplomatic mission of Mexico in Spain', '567f27e2-8885-46dd-acb2-dd53d11c890e', 'a1000000-0000-0000-0000-000000000001', 40.41610000, -3.69730000, 'ea693ddc-74a0-4680-bf62-0823dbf42ce9'),
  -- Israel Embassy in Turkey
  ('b2000000-0000-0000-0000-000000000008', 'Embassy of Israel in Turkey', '+90 312 457 3600', 'info@ankara.mfa.gov.il', 'Mahatma Gandhi Cd. 85, Gaziosmanpasa, Ankara', 'Diplomatic mission of Israel in Turkey', '9ce4f31b-7dd9-4b52-88b7-e3595096444a', 'a1000000-0000-0000-0000-000000000001', 39.90740000, 32.83840000, '6de9c02b-c30f-4c74-8217-d3ccdf876542'),
  -- Nigeria Embassy in USA
  ('b2000000-0000-0000-0000-000000000009', 'Embassy of Nigeria in USA', '+1 202 986 8400', 'info@nigeriaembassyusa.org', '3519 International Court NW, Washington DC', 'Diplomatic mission of Nigeria in the United States', 'f6ec0017-b817-45d1-aa2d-910053a14fe5', 'a1000000-0000-0000-0000-000000000001', 38.95670000, -77.06180000, 'fbf576bf-a760-47fb-bae7-751470acc607'),
  -- Egypt Embassy in UK
  ('b2000000-0000-0000-0000-000000000010', 'Embassy of Egypt in United Kingdom', '+44 20 7499 3304', 'embassy@egyptianconsulate.co.uk', '26 South Street, London W1K 1DW', 'Diplomatic mission of Egypt in the UK', 'fd172532-f9d4-43f7-9bb8-55a8a25b4fb9', 'a1000000-0000-0000-0000-000000000001', 51.50870000, -0.14870000, '7ae8acad-671c-4b31-af7a-2526826af590'),

  -- === CONSULATES ===
  -- India Consulate in Germany (Munich)
  ('b2000000-0000-0000-0000-000000000011', 'Consulate General of India in Munich', '+49 89 210 23 90', 'cg.munich@mea.gov.in', 'Widenmayerstrasse 15, 80538 Munich', 'Indian consular services in southern Germany', '8c015571-4f25-4bc6-b939-c37fbd3ed764', 'a1000000-0000-0000-0000-000000000002', 48.14510000, 11.59230000, '726c7255-42d8-40a5-891f-ea1263d73bc9'),
  -- Brazil Consulate in USA (New York)
  ('b2000000-0000-0000-0000-000000000012', 'Consulate General of Brazil in New York', '+1 212 827 0976', 'cg.novayork@itamaraty.gov.br', '225 E 41st St, New York, NY 10017', 'Brazilian consular services in New York area', 'f6ec0017-b817-45d1-aa2d-910053a14fe5', 'a1000000-0000-0000-0000-000000000002', 40.74930000, -73.97330000, '17fca769-c38e-4a5b-92af-a1ec48576957'),
  -- China Consulate in USA (San Francisco)
  ('b2000000-0000-0000-0000-000000000013', 'Consulate General of China in San Francisco', '+1 415 674 2900', 'chinaconsul_sf@mfa.gov.cn', '1450 Laguna St, San Francisco, CA 94115', 'Chinese consular services on US West Coast', 'f6ec0017-b817-45d1-aa2d-910053a14fe5', 'a1000000-0000-0000-0000-000000000002', 37.78550000, NULL, '808ac4a6-6994-4527-8935-0b2557d96837'),
  -- South Korea Consulate in Japan (Osaka)
  ('b2000000-0000-0000-0000-000000000014', 'Consulate General of South Korea in Osaka', '+81 6 6213 1401', 'osaka@mofa.go.kr', '2-3-4 Nishishinsaibashi, Chuo-ku, Osaka', 'Korean consular services in western Japan', '5f87d98b-c0de-4be1-a109-13542a5e3c84', 'a1000000-0000-0000-0000-000000000002', 34.67200000, NULL, '07be8c59-3289-4ca4-bbf5-f07c6de276ef'),
  -- Argentina Consulate in Spain (Barcelona)
  ('b2000000-0000-0000-0000-000000000015', 'Consulate of Argentina in Barcelona', '+34 93 342 6780', 'cbarc@mrecic.gov.ar', 'Passeig de Gracia 11, 08007 Barcelona', 'Argentine consular services in Catalonia', '567f27e2-8885-46dd-acb2-dd53d11c890e', 'a1000000-0000-0000-0000-000000000002', 41.38910000, 2.16960000, '39441314-5282-4502-bad5-0ca17923287a'),

  -- === BUSINESSES ===
  -- Singapore fintech in UK
  ('b2000000-0000-0000-0000-000000000016', 'Grab Financial Group UK Ltd', '+44 20 7946 0958', 'uk@grab.com', '10 Finsbury Square, London EC2A 1AF', 'Southeast Asian fintech and digital services company', 'fd172532-f9d4-43f7-9bb8-55a8a25b4fb9', 'a1000000-0000-0000-0000-000000000003', 51.52100000, -0.08670000, 'bdd7fb80-d75e-406e-bd29-7a36a66f7383'),
  -- German automotive in USA
  ('b2000000-0000-0000-0000-000000000017', 'BMW North America LLC', '+1 201 307 4000', 'info@bmwna.com', '300 Chestnut Ridge Rd, Woodcliff Lake, NJ 07677', 'German automotive manufacturer North American HQ', 'f6ec0017-b817-45d1-aa2d-910053a14fe5', 'a1000000-0000-0000-0000-000000000003', 41.02450000, -74.05270000, '8c015571-4f25-4bc6-b939-c37fbd3ed764'),
  -- Swiss pharma in India
  ('b2000000-0000-0000-0000-000000000018', 'Novartis India Limited', '+91 22 2496 2000', 'india.info@novartis.com', 'Sandoz House, Shivsagar Estate, Dr. A. Beasant Road, Mumbai', 'Global pharmaceutical company Indian operations', '726c7255-42d8-40a5-891f-ea1263d73bc9', 'a1000000-0000-0000-0000-000000000003', 18.97150000, 72.81650000, '37608878-0cb1-4428-84d5-49a0dbabce71'),
  -- South Korean tech in Vietnam
  ('b2000000-0000-0000-0000-000000000019', 'Samsung Electronics Vietnam', '+84 222 369 9999', 'info@samsung.com.vn', 'Yen Phong IP, Bac Ninh Province', 'Samsung smartphone manufacturing and R&D center', '9a5979ef-1653-4470-b2c8-dadf0fcb7381', 'a1000000-0000-0000-0000-000000000003', 21.17220000, NULL, '07be8c59-3289-4ca4-bbf5-f07c6de276ef'),
  -- Israeli cybersecurity in Germany
  ('b2000000-0000-0000-0000-000000000020', 'CyberArk Software GmbH', '+49 211 931 780', 'germany@cyberark.com', 'Niederkasseler Lohweg 175, 40547 Dusseldorf', 'Israeli cybersecurity company German office', '8c015571-4f25-4bc6-b939-c37fbd3ed764', 'a1000000-0000-0000-0000-000000000003', 51.24700000, 6.72000000, '6de9c02b-c30f-4c74-8217-d3ccdf876542'),
  -- UAE logistics in Singapore
  ('b2000000-0000-0000-0000-000000000021', 'DP World Singapore', '+65 6861 4600', 'singapore@dpworld.com', '10 Harbour Drive, Pasir Panjang Terminal', 'Port and logistics operator from UAE', 'bdd7fb80-d75e-406e-bd29-7a36a66f7383', 'a1000000-0000-0000-0000-000000000003', 1.27300000, NULL, '3292614f-0e99-42f8-b188-0ece2ea32f5a'),
  -- Mexican restaurant chain in Poland
  ('b2000000-0000-0000-0000-000000000022', 'Casa Mexico Warsaw', '+48 22 831 4455', 'reservations@casamexicowarsaw.pl', 'ul. Foksal 17, 00-372 Warsaw', 'Authentic Mexican restaurant and cultural space', '7b399d0a-caf9-44b0-8ac5-d054fbacbb4e', 'a1000000-0000-0000-0000-000000000003', 52.23300000, 21.02350000, 'ea693ddc-74a0-4680-bf62-0823dbf42ce9'),

  -- === NGOs ===
  -- Canadian humanitarian in Kenya
  ('b2000000-0000-0000-0000-000000000023', 'World Vision Kenya', '+254 20 509 2500', 'kenya@wvi.org', 'Karen Road, Off Ngong Road, Nairobi', 'International humanitarian organization Kenya office', '4250abff-f03a-468b-b1d7-3bb69cc9e2dd', 'a1000000-0000-0000-0000-000000000004', -1.30380000, 36.76800000, '955d8e2a-d3d5-48b3-b2c0-0b02d5fe49d0'),
  -- German environmental NGO in Brazil
  ('b2000000-0000-0000-0000-000000000024', 'GIZ Brazil - Deutsche Gesellschaft', '+55 61 2101 2170', 'giz-brasilien@giz.de', 'SCN Quadra 1, Bloco C, Sala 1501, Brasilia', 'German development and sustainability cooperation', '17fca769-c38e-4a5b-92af-a1ec48576957', 'a1000000-0000-0000-0000-000000000004', -15.78890000, -47.87850000, '8c015571-4f25-4bc6-b939-c37fbd3ed764'),
  -- US medical NGO in Nigeria
  ('b2000000-0000-0000-0000-000000000025', 'Doctors Without Borders Nigeria', '+234 1 791 5040', 'msf-nigeria@msf.org', '4 Onitsha Crescent, Off Adeola Odeku, Lagos', 'Medical humanitarian organization Nigerian operations', 'fbf576bf-a760-47fb-bae7-751470acc607', 'a1000000-0000-0000-0000-000000000004', 6.43190000, 3.42160000, 'f6ec0017-b817-45d1-aa2d-910053a14fe5'),
  -- South African wildlife conservation
  ('b2000000-0000-0000-0000-000000000026', 'WWF South Africa', '+27 21 657 6600', 'info@wwf.org.za', '1st Floor, Bridge House, Boundary Terraces, Cape Town', 'Global wildlife conservation organization SA branch', 'e0d4a87e-a501-4011-a113-33708f87c31e', 'a1000000-0000-0000-0000-000000000004', -33.97300000, 18.46330000, '37608878-0cb1-4428-84d5-49a0dbabce71'),

  -- === CULTURAL CENTERS ===
  -- British Council in India
  ('b2000000-0000-0000-0000-000000000027', 'British Council New Delhi', '+91 11 4149 7350', 'newdelhi@britishcouncil.org', '17 Kasturba Gandhi Marg, New Delhi 110001', 'British cultural relations and educational opportunities', '726c7255-42d8-40a5-891f-ea1263d73bc9', 'a1000000-0000-0000-0000-000000000005', 28.63130000, 77.22220000, 'fd172532-f9d4-43f7-9bb8-55a8a25b4fb9'),
  -- Goethe Institut in Brazil
  ('b2000000-0000-0000-0000-000000000028', 'Goethe-Institut Sao Paulo', '+55 11 3296 7000', 'info@saopaulo.goethe.org', 'Rua Lisboa 974, Pinheiros, Sao Paulo', 'German cultural institute promoting language and culture', '17fca769-c38e-4a5b-92af-a1ec48576957', 'a1000000-0000-0000-0000-000000000005', -23.55810000, -46.68320000, '8c015571-4f25-4bc6-b939-c37fbd3ed764'),
  -- Alliance Francaise in Australia
  ('b2000000-0000-0000-0000-000000000029', 'Alliance Francaise de Sydney', '+61 2 9292 5700', 'info@afsydney.com.au', '257 Clarence St, Sydney NSW 2000', 'French cultural center promoting language and arts', 'ec9f55d7-f2fe-4fdb-a937-1509e3c24e0b', 'a1000000-0000-0000-0000-000000000005', -33.87300000, NULL, 'f790947b-1621-4e5d-93fb-d7f2a622dc56'),
  -- Confucius Institute in UK
  ('b2000000-0000-0000-0000-000000000030', 'Confucius Institute London', '+44 20 7911 5000', 'confucius@soas.ac.uk', 'SOAS University of London, Thornhaugh St, WC1H 0XG', 'Chinese cultural and language education center', 'fd172532-f9d4-43f7-9bb8-55a8a25b4fb9', 'a1000000-0000-0000-0000-000000000005', 51.52260000, -0.12960000, '808ac4a6-6994-4527-8935-0b2557d96837'),

  -- === TRADE OFFICES (new type) ===
  -- Korean trade office in Germany
  ('b2000000-0000-0000-0000-000000000031', 'KOTRA Berlin Trade Center', '+49 30 2639 6340', 'berlin@kotra.or.kr', 'Friedrichstrasse 187, 10117 Berlin', 'Korean Trade-Investment Promotion Agency Berlin office', '8c015571-4f25-4bc6-b939-c37fbd3ed764', 'a1000000-0000-0000-0000-000000000006', 52.52610000, 13.38700000, '07be8c59-3289-4ca4-bbf5-f07c6de276ef'),
  -- Japanese trade office in UK
  ('b2000000-0000-0000-0000-000000000032', 'JETRO London', '+44 20 7421 8300', 'lon@jetro.go.jp', '8th Floor, MidCity Place, 71 High Holborn, London WC1V 6AL', 'Japan External Trade Organization London office', 'fd172532-f9d4-43f7-9bb8-55a8a25b4fb9', 'a1000000-0000-0000-0000-000000000006', 51.51750000, -0.11820000, '5f87d98b-c0de-4be1-a109-13542a5e3c84'),
  -- Indian trade office in USA
  ('b2000000-0000-0000-0000-000000000033', 'India Trade Promotion Organisation', '+1 212 586 4901', 'itpony@aol.com', '1270 Avenue of the Americas, Suite 1810, New York', 'Indian trade promotion and exhibition services', 'f6ec0017-b817-45d1-aa2d-910053a14fe5', 'a1000000-0000-0000-0000-000000000006', 40.76010000, -73.97970000, '726c7255-42d8-40a5-891f-ea1263d73bc9'),
  -- UAE trade office in Poland
  ('b2000000-0000-0000-0000-000000000034', 'Dubai Chamber Warsaw', '+48 22 299 1234', 'warsaw@dubaichamber.com', 'Zlota 59, 00-120 Warsaw', 'Dubai Chamber of Commerce Warsaw representative', '7b399d0a-caf9-44b0-8ac5-d054fbacbb4e', 'a1000000-0000-0000-0000-000000000006', 52.22900000, 21.00350000, '3292614f-0e99-42f8-b188-0ece2ea32f5a'),

  -- === RESEARCH INSTITUTES (new type) ===
  -- German research in USA
  ('b2000000-0000-0000-0000-000000000035', 'Max Planck Society Washington', '+1 202 296 7422', 'info@maxplanckflorida.org', '1156 15th St NW, Suite 510, Washington DC', 'German scientific research organization US liaison', 'f6ec0017-b817-45d1-aa2d-910053a14fe5', 'a1000000-0000-0000-0000-000000000007', 38.90470000, -77.03470000, '8c015571-4f25-4bc6-b939-c37fbd3ed764'),
  -- Japanese research in UK
  ('b2000000-0000-0000-0000-000000000036', 'RIKEN UK Research Hub', '+44 20 7580 1234', 'uk@riken.jp', 'Darwin Building, Gower Street, London WC1E 6BT', 'Japanese scientific research institute UK office', 'fd172532-f9d4-43f7-9bb8-55a8a25b4fb9', 'a1000000-0000-0000-0000-000000000007', 51.52490000, -0.13490000, '5f87d98b-c0de-4be1-a109-13542a5e3c84'),
  -- Chinese research in Australia
  ('b2000000-0000-0000-0000-000000000037', 'Chinese Academy of Sciences Melbourne', '+61 3 9035 5511', 'cas.melbourne@cas.cn', '187 Grattan St, Carlton VIC 3053, Melbourne', 'Chinese science and technology research center', 'ec9f55d7-f2fe-4fdb-a937-1509e3c24e0b', 'a1000000-0000-0000-0000-000000000007', -37.80030000, NULL, '808ac4a6-6994-4527-8935-0b2557d96837'),

  -- === INTERNATIONAL SCHOOLS (new type) ===
  -- British school in Thailand
  ('b2000000-0000-0000-0000-000000000038', 'British International School Bangkok', '+66 2 963 5900', 'admissions@bkkprep.ac.th', '36 Sukhumvit 53, Khlong Toei Nuea, Bangkok', 'British curriculum international school in Bangkok', '6568352c-2f34-450f-9b3f-7a91229a11c4', 'a1000000-0000-0000-0000-000000000008', 13.72560000, NULL, 'fd172532-f9d4-43f7-9bb8-55a8a25b4fb9'),
  -- American school in Singapore
  ('b2000000-0000-0000-0000-000000000039', 'Singapore American School', '+65 6363 3403', 'admissions@sas.edu.sg', '40 Woodlands St 41, Singapore 738547', 'American curriculum K-12 school in Singapore', 'bdd7fb80-d75e-406e-bd29-7a36a66f7383', 'a1000000-0000-0000-0000-000000000008', 1.44000000, NULL, 'f6ec0017-b817-45d1-aa2d-910053a14fe5'),
  -- German school in Turkey (Istanbul)
  ('b2000000-0000-0000-0000-000000000040', 'Deutsche Schule Istanbul', '+90 212 245 4526', 'info@ds-istanbul.de', 'Sahkulu Mah, Sehbender Sok 18, Beyoglu, Istanbul', 'German international school in Istanbul', '9ce4f31b-7dd9-4b52-88b7-e3595096444a', 'a1000000-0000-0000-0000-000000000008', 41.02710000, 28.97420000, '8c015571-4f25-4bc6-b939-c37fbd3ed764');

-- =============================================
-- Country Connections (20 new entries)
-- =============================================
INSERT INTO countries_connections (id, embassy_org_id, consulate_org_id, common_info, location_country_id) VALUES
  -- Brazil → Germany
  ('c2000000-0000-0000-0000-000000000001', 'b2000000-0000-0000-0000-000000000001', NULL, 'Brazilian diplomatic mission in Germany. Embassy in Berlin.', '8c015571-4f25-4bc6-b939-c37fbd3ed764'),
  -- India → USA (embassy + consulate not linked since different cities)
  ('c2000000-0000-0000-0000-000000000002', 'b2000000-0000-0000-0000-000000000002', NULL, 'Indian diplomatic mission in the United States. Embassy in Washington DC.', 'f6ec0017-b817-45d1-aa2d-910053a14fe5'),
  -- China → UK
  ('c2000000-0000-0000-0000-000000000003', 'b2000000-0000-0000-0000-000000000003', NULL, 'Chinese diplomatic mission in the United Kingdom. Embassy in London.', 'fd172532-f9d4-43f7-9bb8-55a8a25b4fb9'),
  -- China → USA (consulate only)
  ('c2000000-0000-0000-0000-000000000004', NULL, 'b2000000-0000-0000-0000-000000000013', 'Chinese consular services on the US West Coast. Consulate General in San Francisco.', 'f6ec0017-b817-45d1-aa2d-910053a14fe5'),
  -- Australia → Japan
  ('c2000000-0000-0000-0000-000000000005', 'b2000000-0000-0000-0000-000000000004', NULL, 'Australian diplomatic mission in Japan. Embassy in Tokyo.', '5f87d98b-c0de-4be1-a109-13542a5e3c84'),
  -- South Korea → Germany
  ('c2000000-0000-0000-0000-000000000006', 'b2000000-0000-0000-0000-000000000005', NULL, 'South Korean diplomatic mission in Germany. Embassy in Berlin.', '8c015571-4f25-4bc6-b939-c37fbd3ed764'),
  -- South Korea → Japan
  ('c2000000-0000-0000-0000-000000000007', NULL, 'b2000000-0000-0000-0000-000000000014', 'South Korean consular presence in Japan. Consulate General in Osaka.', '5f87d98b-c0de-4be1-a109-13542a5e3c84'),
  -- Canada → Italy
  ('c2000000-0000-0000-0000-000000000008', 'b2000000-0000-0000-0000-000000000006', NULL, 'Canadian diplomatic mission in Italy. Embassy in Rome.', 'f790947b-1621-4e5d-93fb-d7f2a622dc56'),
  -- Mexico → Spain (embassy + consulate)
  ('c2000000-0000-0000-0000-000000000009', 'b2000000-0000-0000-0000-000000000007', NULL, 'Mexican diplomatic mission in Spain. Embassy in Madrid.', '567f27e2-8885-46dd-acb2-dd53d11c890e'),
  -- Argentina → Spain
  ('c2000000-0000-0000-0000-000000000010', NULL, 'b2000000-0000-0000-0000-000000000015', 'Argentine consular services in Spain. Consulate in Barcelona.', '567f27e2-8885-46dd-acb2-dd53d11c890e'),
  -- Israel → Turkey
  ('c2000000-0000-0000-0000-000000000011', 'b2000000-0000-0000-0000-000000000008', NULL, 'Israeli diplomatic mission in Turkey. Embassy in Ankara.', '9ce4f31b-7dd9-4b52-88b7-e3595096444a'),
  -- Nigeria → USA
  ('c2000000-0000-0000-0000-000000000012', 'b2000000-0000-0000-0000-000000000009', NULL, 'Nigerian diplomatic mission in the United States. Embassy in Washington DC.', 'f6ec0017-b817-45d1-aa2d-910053a14fe5'),
  -- Egypt → UK
  ('c2000000-0000-0000-0000-000000000013', 'b2000000-0000-0000-0000-000000000010', NULL, 'Egyptian diplomatic mission in the United Kingdom. Embassy in London.', 'fd172532-f9d4-43f7-9bb8-55a8a25b4fb9'),
  -- India → Germany (consulate)
  ('c2000000-0000-0000-0000-000000000014', NULL, 'b2000000-0000-0000-0000-000000000011', 'Indian consular services in southern Germany. Consulate General in Munich.', '8c015571-4f25-4bc6-b939-c37fbd3ed764'),
  -- Brazil → USA (consulate)
  ('c2000000-0000-0000-0000-000000000015', NULL, 'b2000000-0000-0000-0000-000000000012', 'Brazilian consular services in the eastern United States. Consulate General in New York.', 'f6ec0017-b817-45d1-aa2d-910053a14fe5'),
  -- UK → India (cultural center)
  ('c2000000-0000-0000-0000-000000000016', NULL, NULL, 'British cultural and educational presence in India through the British Council in New Delhi.', '726c7255-42d8-40a5-891f-ea1263d73bc9'),
  -- Germany → Brazil (cultural center)
  ('c2000000-0000-0000-0000-000000000017', NULL, NULL, 'German cultural presence in Brazil through Goethe-Institut in Sao Paulo.', '17fca769-c38e-4a5b-92af-a1ec48576957'),
  -- South Korea → Vietnam (business)
  ('c2000000-0000-0000-0000-000000000018', NULL, NULL, 'Strong South Korean business presence in Vietnam, led by Samsung manufacturing operations.', '9a5979ef-1653-4470-b2c8-dadf0fcb7381'),
  -- UAE → Singapore (trade)
  ('c2000000-0000-0000-0000-000000000019', NULL, NULL, 'UAE trade and logistics presence in Singapore through DP World port operations.', 'bdd7fb80-d75e-406e-bd29-7a36a66f7383'),
  -- Germany → Turkey (school)
  ('c2000000-0000-0000-0000-000000000020', NULL, NULL, 'German educational presence in Turkey through Deutsche Schule Istanbul.', '9ce4f31b-7dd9-4b52-88b7-e3595096444a');

COMMIT;
