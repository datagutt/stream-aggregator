-- Communities: brandable filter recipes for the frontend.
-- Each community curates a slice of the global stream pool via the `filter` JSON
-- recipe, and ships its own brand (name, accent, theme, logo).
CREATE TABLE communities (
    slug              TEXT PRIMARY KEY NOT NULL,
    name              TEXT NOT NULL,
    tagline           TEXT,
    accent            TEXT NOT NULL,
    accent_contrast   TEXT,
    logo_url          TEXT,
    default_theme     TEXT NOT NULL DEFAULT 'dark' CHECK (default_theme IN ('dark','light')),
    filter            TEXT NOT NULL DEFAULT '{}',
    about_md          TEXT,
    created_at        TEXT NOT NULL,
    updated_at        TEXT NOT NULL
);

CREATE INDEX idx_communities_updated_at ON communities(updated_at);
