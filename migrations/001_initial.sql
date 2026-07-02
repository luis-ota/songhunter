CREATE TABLE IF NOT EXISTS audio_cache (
    audio_hash TEXT PRIMARY KEY,
    songs TEXT NOT NULL,
    created_at DATETIME NOT NULL
);

CREATE TABLE IF NOT EXISTS task_cache (
    task_id TEXT PRIMARY KEY,
    payload TEXT NOT NULL,
    updated_at DATETIME NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_audio_cache_created ON audio_cache(created_at);
