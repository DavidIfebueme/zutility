ALTER TABLE deposit_addresses
    ADD COLUMN IF NOT EXISTS network TEXT NOT NULL DEFAULT 'unknown';

-- Backfill existing addresses by inspecting Zcash address prefixes.
-- Testnet unified addresses start with 'utest', Sapling testnet with 'ztestsapling',
-- transparent testnet P2PKH with 'tm'. Mainnet unified starts with 'u1', Sapling
-- with 'zs1', transparent P2PKH with 't1'.
UPDATE deposit_addresses
SET network = CASE
    WHEN address LIKE 'utest%' OR address LIKE 'ztestsapling%' OR address LIKE 'tm%' OR address LIKE 'uregtest%' THEN 'testnet'
    WHEN address LIKE 'u1%' OR address LIKE 'zs1%' OR address LIKE 't1%' OR address LIKE 't3%' THEN 'mainnet'
    ELSE 'unknown'
END
WHERE network = 'unknown';

DROP INDEX IF EXISTS idx_deposit_addresses_unused_type;

CREATE INDEX idx_deposit_addresses_unused_type_network
    ON deposit_addresses (address_type, used, network)
    WHERE used = false;
