-- Community-to-domain mapping. host is the primary key so DB-level uniqueness
-- guarantees one community per hostname (the frontend middleware relies on this
-- when resolving Host -> community slug).
CREATE TABLE community_domains (
    host              TEXT PRIMARY KEY NOT NULL,
    slug              TEXT NOT NULL REFERENCES communities(slug) ON DELETE CASCADE,
    created_at        TEXT NOT NULL
);

CREATE INDEX idx_community_domains_slug ON community_domains(slug);
