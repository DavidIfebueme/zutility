CREATE TABLE currencies (
    code CHAR(3) PRIMARY KEY,
    exponent SMALLINT NOT NULL CHECK (exponent >= 0 AND exponent <= 4),
    symbol TEXT NOT NULL,
    locale TEXT NOT NULL,
    name TEXT NOT NULL
);

INSERT INTO currencies (code, exponent, symbol, locale, name) VALUES
    ('NGN', 2, '₦', 'en-NG', 'Naira'),
    ('XOF', 0, 'CFA', 'fr-SN', 'West African CFA franc'),
    ('GMD', 2, 'D', 'en-GM', 'Dalasi'),
    ('MWK', 2, 'MK', 'en-MW', 'Kwacha'),
    ('ZMW', 2, 'K', 'en-ZM', 'Zambian kwacha'),
    ('ZAR', 2, 'R', 'en-ZA', 'Rand'),
    ('USD', 2, '$', 'en-US', 'US dollar'),
    ('KES', 2, 'KSh', 'en-KE', 'Kenyan shilling'),
    ('GHS', 2, 'GH₵', 'en-GH', 'Cedi'),
    ('EGP', 2, 'E£', 'ar-EG', 'Egyptian pound');

ALTER TABLE users
    ADD COLUMN preferred_currency CHAR(3) NOT NULL DEFAULT 'NGN'
    REFERENCES currencies(code);
