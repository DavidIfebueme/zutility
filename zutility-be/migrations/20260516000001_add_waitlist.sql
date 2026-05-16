CREATE TABLE IF NOT EXISTS waitlist_entries (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email TEXT NOT NULL,
    display_name TEXT,
    email_verified BOOLEAN NOT NULL DEFAULT false,
    referral_code TEXT NOT NULL UNIQUE,
    referred_by TEXT REFERENCES waitlist_entries(referral_code),
    ip_hash TEXT,
    utm_source TEXT,
    utm_medium TEXT,
    utm_campaign TEXT,
    utm_content TEXT,
    utm_term TEXT,
    verified_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_waitlist_entries_email_lower ON waitlist_entries (LOWER(email));
CREATE INDEX IF NOT EXISTS idx_waitlist_entries_referral_code ON waitlist_entries (referral_code);
CREATE INDEX IF NOT EXISTS idx_waitlist_entries_created_at ON waitlist_entries (created_at);

CREATE TABLE IF NOT EXISTS waitlist_verify_tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entry_id UUID NOT NULL REFERENCES waitlist_entries(id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    used_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_waitlist_verify_tokens_hash ON waitlist_verify_tokens (token_hash) WHERE used_at IS NULL;
