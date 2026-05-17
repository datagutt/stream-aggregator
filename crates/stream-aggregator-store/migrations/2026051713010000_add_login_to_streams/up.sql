-- URL-safe login/handle alongside the internal user_id. Providers populate
-- it from the upstream platform (e.g. Twitch's user_login). Existing rows
-- are NULL until the next scrape refreshes them.
ALTER TABLE streams ADD COLUMN login TEXT;
