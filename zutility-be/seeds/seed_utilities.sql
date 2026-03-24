INSERT INTO utilities (slug, utility_type, name, logo_url, field_config, active)
VALUES
  ('mtn', 'airtime', 'MTN Airtime', NULL, '{"service_ref": "phone", "amount_min_kobo": 5000}', true),
  ('airtel', 'airtime', 'Airtel Airtime', NULL, '{"service_ref": "phone", "amount_min_kobo": 5000}', true),
  ('glo', 'airtime', 'Glo Airtime', NULL, '{"service_ref": "phone", "amount_min_kobo": 5000}', true),
  ('9mobile', 'airtime', '9mobile Airtime', NULL, '{"service_ref": "phone", "amount_min_kobo": 5000}', true),
  ('dstv', 'dstv', 'DSTV', NULL, '{"service_ref": "smartcard", "amount_min_kobo": 10000}', true),
  ('gotv', 'gotv', 'GOtv', NULL, '{"service_ref": "smartcard", "amount_min_kobo": 10000}', true),
  ('phcn', 'electricity', 'Electricity', NULL, '{"service_ref": "meter", "amount_min_kobo": 50000}', true)
ON CONFLICT (slug) DO UPDATE
SET
  utility_type = EXCLUDED.utility_type,
  name = EXCLUDED.name,
  logo_url = EXCLUDED.logo_url,
  field_config = EXCLUDED.field_config,
  active = EXCLUDED.active;
