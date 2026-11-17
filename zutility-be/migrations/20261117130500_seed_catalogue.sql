INSERT INTO countries (iso2, iso3, name, currency, dial_code, timezone, enabled) VALUES
    ('NG', 'NGA', 'Nigeria', 'NGN', '+234', 'Africa/Lagos', true),
    ('SN', 'SEN', 'Senegal', 'XOF', '+221', 'Africa/Dakar', false),
    ('GM', 'GMB', 'Gambia', 'GMD', '+220', 'Africa/Banjul', false),
    ('ML', 'MLI', 'Mali', 'XOF', '+223', 'Africa/Bamako', false),
    ('BJ', 'BEN', 'Benin', 'XOF', '+229', 'Africa/Porto-Novo', false),
    ('BF', 'BFA', 'Burkina Faso', 'XOF', '+226', 'Africa/Ouagadougou', false),
    ('CI', 'CIV', 'Cote d''Ivoire', 'XOF', '+225', 'Africa/Abidjan', false),
    ('TG', 'TGO', 'Togo', 'XOF', '+228', 'Africa/Lome', false),
    ('MW', 'MWI', 'Malawi', 'MWK', '+265', 'Africa/Blantyre', false),
    ('ZW', 'ZWE', 'Zimbabwe', 'USD', '+263', 'Africa/Harare', false),
    ('ZM', 'ZMB', 'Zambia', 'ZMW', '+260', 'Africa/Lusaka', false),
    ('ZA', 'ZAF', 'South Africa', 'ZAR', '+27', 'Africa/Johannesburg', false);

INSERT INTO provider_products (
    id, provider, failover_provider, provider_product_id, country_iso2, utility_type,
    utility_category, slug, name, descriptor, icon_key, service_ref_label,
    service_ref_placeholder, service_ref_pattern, amount_kind, amount_min, amount_max,
    destination_currency, fulfilment_kind, wholesale_rate, wholesale_fee, retail_rate,
    retail_fee, operator_name, enabled
) VALUES
    ('45468ce6-d81a-52eb-b03d-ff462b30aae9', 'inlomax', 'vtpass', 'mtn', 'NG', 'airtime',
     'airtime', 'mtn', 'MTN Airtime', 'MTN airtime top-up', 'airtime', 'Phone Number',
     '08012345678', '^0[7-9][0-1][0-9]{8}$', 'ranged', 50, 50000,
     'NGN', 'payment', 1, 0, NULL, 0, 'MTN', true),

    ('1566fd6a-ba03-52e3-a2ef-7c7b5cf496bd', 'inlomax', 'vtpass', 'airtel', 'NG', 'airtime',
     'airtime', 'airtel', 'Airtel Airtime', 'Airtel airtime top-up', 'airtime', 'Phone Number',
     '08012345678', '^0[7-9][0-1][0-9]{8}$', 'ranged', 50, 50000,
     'NGN', 'payment', 1, 0, NULL, 0, 'Airtel', true),

    ('2bacec06-9ef1-5d83-978a-f7508a2fc600', 'inlomax', 'vtpass', 'glo', 'NG', 'airtime',
     'airtime', 'glo', 'Glo Airtime', 'Glo airtime top-up', 'airtime', 'Phone Number',
     '08012345678', '^0[7-9][0-1][0-9]{8}$', 'ranged', 50, 50000,
     'NGN', 'payment', 1, 0, NULL, 0, 'Glo', true),

    ('ed4b2e6d-6c0a-50c4-a0ba-7a82278d1563', 'inlomax', 'vtpass', '9mobile', 'NG', 'airtime',
     'airtime', '9mobile', '9mobile Airtime', '9mobile airtime top-up', 'airtime', 'Phone Number',
     '08012345678', '^0[7-9][0-1][0-9]{8}$', 'ranged', 50, 50000,
     'NGN', 'payment', 1, 0, NULL, 0, '9mobile', true),

    ('5b4afc0c-f7c0-5697-87db-8ddd52d3e4b7', 'inlomax', 'vtpass', 'mtn-data', 'NG', 'data',
     'data', 'mtn-data', 'MTN Data', 'MTN data bundle', 'data', 'Phone Number',
     '08012345678', '^0[7-9][0-1][0-9]{8}$', 'variation', NULL, NULL,
     'NGN', 'pin', 1, 0, NULL, 0, 'MTN', true),

    ('51c817ca-6f88-51e3-90ed-7fb5fa892378', 'inlomax', 'vtpass', 'airtel-data', 'NG', 'data',
     'data', 'airtel-data', 'Airtel Data', 'Airtel data bundle', 'data', 'Phone Number',
     '08012345678', '^0[7-9][0-1][0-9]{8}$', 'variation', NULL, NULL,
     'NGN', 'pin', 1, 0, NULL, 0, 'Airtel', true),

    ('5ebd983c-2e7c-54cb-879d-88e0a12681e5', 'inlomax', 'vtpass', 'glo-data', 'NG', 'data',
     'data', 'glo-data', 'Glo Data', 'Glo data bundle', 'data', 'Phone Number',
     '08012345678', '^0[7-9][0-1][0-9]{8}$', 'variation', NULL, NULL,
     'NGN', 'pin', 1, 0, NULL, 0, 'Glo', true),

    ('1e85c504-b080-57c8-b25f-ae78041de27d', 'inlomax', 'vtpass', '9mobile-data', 'NG', 'data',
     'data', '9mobile-data', '9mobile Data', '9mobile data bundle', 'data', 'Phone Number',
     '08012345678', '^0[7-9][0-1][0-9]{8}$', 'variation', NULL, NULL,
     'NGN', 'pin', 1, 0, NULL, 0, '9mobile', true),

    ('2cbe96ab-6ca0-5b10-ad4d-34fb114f63f8', 'inlomax', 'vtpass', 'dstv', 'NG', 'dstv',
     'television', 'dstv', 'DSTV', 'DSTV subscription', 'tv', 'Smartcard Number',
     'e.g. 2012345678', '^[0-9]{10}$', 'variation', NULL, NULL,
     'NGN', 'payment', 1, 0, NULL, 0, 'DSTV', true),

    ('afe77f13-7aca-5e7e-a6c1-7462f1b4ca39', 'inlomax', 'vtpass', 'gotv', 'NG', 'gotv',
     'television', 'gotv', 'GOtv', 'GOtv subscription', 'tv', 'Smartcard Number',
     'e.g. 2012345678', '^[0-9]{10}$', 'variation', NULL, NULL,
     'NGN', 'payment', 1, 0, NULL, 0, 'GOtv', true),

    ('de4ccde2-e92e-5229-a75e-2d783531b169', 'inlomax', 'vtpass', 'startimes', 'NG', 'startimes',
     'television', 'startimes', 'Startimes', 'Startimes subscription', 'tv', 'Smartcard Number',
     'e.g. 2012345678', '^[0-9]{10}$', 'variation', NULL, NULL,
     'NGN', 'payment', 1, 0, NULL, 0, 'Startimes', true),

    ('2e684ba4-937c-5c63-a0a2-b80947a50503', 'vtpass', NULL, 'showmax', 'NG', 'showmax',
     'television', 'showmax', 'Showmax', 'Showmax subscription', 'tv', 'Email or Phone',
     'you@email.com', NULL, 'variation', NULL, NULL,
     'NGN', 'payment', 1, 0, NULL, 0, 'Showmax', true),

    ('af0d113f-1fce-56cc-9d46-971938798d4f', 'inlomax', 'vtpass', 'ikeja-electric', 'NG', 'electricity',
     'electricity', 'ikeja-electric', 'Ikeja Electric (IKEDC)', 'Ikeja Electric prepaid token', 'electricity', 'Meter Number',
     'e.g. 54012345678', '^[0-9]{11,13}$', 'ranged', 500, 500000,
     'NGN', 'payment', 1, 0, NULL, 0, 'Ikeja Electric', true),

    ('0bcccacc-c14c-5c4c-ba17-4a18f16f985b', 'inlomax', 'vtpass', 'eko-electric', 'NG', 'electricity',
     'electricity', 'eko-electric', 'Eko Electric (EKEDC)', 'Eko Electric prepaid token', 'electricity', 'Meter Number',
     'e.g. 54012345678', '^[0-9]{11,13}$', 'ranged', 500, 500000,
     'NGN', 'payment', 1, 0, NULL, 0, 'Eko Electric', true),

    ('8f3d2f13-cf6f-53bc-8018-ecaa90511c47', 'inlomax', 'vtpass', 'abuja-electric', 'NG', 'electricity',
     'electricity', 'abuja-electric', 'Abuja Electric (AEDC)', 'Abuja Electric prepaid token', 'electricity', 'Meter Number',
     'e.g. 54012345678', '^[0-9]{11,13}$', 'ranged', 500, 500000,
     'NGN', 'payment', 1, 0, NULL, 0, 'Abuja Electric', true),

    ('2fe67e82-a228-58c9-b44b-1b1c1270663f', 'inlomax', 'vtpass', 'ibadan-electric', 'NG', 'electricity',
     'electricity', 'ibadan-electric', 'Ibadan Electric (IBEDC)', 'Ibadan Electric prepaid token', 'electricity', 'Meter Number',
     'e.g. 54012345678', '^[0-9]{11,13}$', 'ranged', 500, 500000,
     'NGN', 'payment', 1, 0, NULL, 0, 'Ibadan Electric', true),

    ('11e77585-13fe-5c8e-934a-43943e888444', 'inlomax', 'vtpass', 'kano-electric', 'NG', 'electricity',
     'electricity', 'kano-electric', 'Kano Electric (KEDCO)', 'Kano Electric prepaid token', 'electricity', 'Meter Number',
     'e.g. 54012345678', '^[0-9]{11,13}$', 'ranged', 500, 500000,
     'NGN', 'payment', 1, 0, NULL, 0, 'Kano Electric', true),

    ('2cb2e723-ceb9-5861-8714-28eba85b176f', 'inlomax', 'vtpass', 'phed-electric', 'NG', 'electricity',
     'electricity', 'phed-electric', 'Port Harcourt Electric (PHED)', 'Port Harcourt Electric prepaid token', 'electricity', 'Meter Number',
     'e.g. 54012345678', '^[0-9]{11,13}$', 'ranged', 500, 500000,
     'NGN', 'payment', 1, 0, NULL, 0, 'Port Harcourt Electric', true),

    ('b3acf191-e587-5b94-9444-ff9d71f8a20f', 'inlomax', 'vtpass', 'jos-electric', 'NG', 'electricity',
     'electricity', 'jos-electric', 'Jos Electric (JED)', 'Jos Electric prepaid token', 'electricity', 'Meter Number',
     'e.g. 54012345678', '^[0-9]{11,13}$', 'ranged', 500, 500000,
     'NGN', 'payment', 1, 0, NULL, 0, 'Jos Electric', true),

    ('270188f6-7327-56f3-b078-dbba36d03463', 'inlomax', 'vtpass', 'kaduna-electric', 'NG', 'electricity',
     'electricity', 'kaduna-electric', 'Kaduna Electric (KAEDCO)', 'Kaduna Electric prepaid token', 'electricity', 'Meter Number',
     'e.g. 54012345678', '^[0-9]{11,13}$', 'ranged', 500, 500000,
     'NGN', 'payment', 1, 0, NULL, 0, 'Kaduna Electric', true),

    ('dc2ca444-fda0-50ab-b50c-e67473b37f07', 'inlomax', 'vtpass', 'enugu-electric', 'NG', 'electricity',
     'electricity', 'enugu-electric', 'Enugu Electric (EEDC)', 'Enugu Electric prepaid token', 'electricity', 'Meter Number',
     'e.g. 54012345678', '^[0-9]{11,13}$', 'ranged', 500, 500000,
     'NGN', 'payment', 1, 0, NULL, 0, 'Enugu Electric', true),

    ('eaa73eaf-f0f2-5abb-89a7-37c1ba71a109', 'inlomax', 'vtpass', 'benin-electric', 'NG', 'electricity',
     'electricity', 'benin-electric', 'Benin Electric (BEDC)', 'Benin Electric prepaid token', 'electricity', 'Meter Number',
     'e.g. 54012345678', '^[0-9]{11,13}$', 'ranged', 500, 500000,
     'NGN', 'payment', 1, 0, NULL, 0, 'Benin Electric', true),

    ('f18b46ce-b142-57f4-b220-7ead21236e0a', 'inlomax', 'vtpass', 'yola-electric', 'NG', 'electricity',
     'electricity', 'yola-electric', 'Yola Electric (YEDC)', 'Yola Electric prepaid token', 'electricity', 'Meter Number',
     'e.g. 54012345678', '^[0-9]{11,13}$', 'ranged', 500, 500000,
     'NGN', 'payment', 1, 0, NULL, 0, 'Yola Electric', true),

    ('baf07691-f099-5841-a81f-2f2fd01921bb', 'inlomax', 'vtpass', 'aba-electric', 'NG', 'electricity',
     'electricity', 'aba-electric', 'Aba Electric (ABA)', 'Aba Electric prepaid token', 'electricity', 'Meter Number',
     'e.g. 54012345678', '^[0-9]{11,13}$', 'ranged', 500, 500000,
     'NGN', 'payment', 1, 0, NULL, 0, 'Aba Electric', true),

    ('b40d28b7-2db0-5c8f-bb49-acd5531c6438', 'inlomax', 'vtpass', 'waec-registration', 'NG', 'waec',
     'education', 'waec-registration', 'WAEC Registration', 'WAEC registration pin', 'education', 'Phone Number',
     '08012345678', '^0[7-9][0-1][0-9]{8}$', 'variation', NULL, NULL,
     'NGN', 'pin', 1, 0, NULL, 0, 'WAEC', true),

    ('9b9a02ba-f8c1-5e4b-b063-848300654f5e', 'inlomax', 'vtpass', 'waec-result-checker', 'NG', 'waec',
     'education', 'waec-result-checker', 'WAEC Result Checker', 'WAEC result checker pin', 'education', 'Phone Number',
     '08012345678', '^0[7-9][0-1][0-9]{8}$', 'variation', NULL, NULL,
     'NGN', 'pin', 1, 0, NULL, 0, 'WAEC', true),

    ('c4d5f216-262f-557b-969d-4b66188d3add', 'inlomax', 'vtpass', 'jamb', 'NG', 'jamb',
     'education', 'jamb', 'JAMB Pin', 'JAMB registration pin', 'education', 'Phone Number',
     '08012345678', '^0[7-9][0-1][0-9]{8}$', 'fixed', 6500, 6500,
     'NGN', 'pin', 1, 0, NULL, 0, 'JAMB', true),

    ('7d3ca8c6-5823-5f99-8f9e-cc7579f5b7af', 'remita_collections', NULL, 'school-fees', 'NG', 'school_fees',
     'school', 'school-fees', 'School Fees', 'School fees payment by Remita RRR', 'school', 'RRR or Student ID',
     'Enter RRR or Student ID', NULL, 'ranged', 500, 5000000,
     'NGN', 'payment', 1, 0, NULL, 0, 'Remita', true);
