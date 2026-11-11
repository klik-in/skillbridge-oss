ALTER TABLE accounts
    ADD COLUMN disabled boolean NOT NULL DEFAULT false,
    ADD COLUMN auth_version bigint NOT NULL DEFAULT 0 CHECK (auth_version >= 0);

CREATE INDEX sessions_account ON sessions (account_id);
