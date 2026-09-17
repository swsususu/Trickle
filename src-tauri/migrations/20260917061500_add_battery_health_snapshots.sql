-- Daily battery health snapshots, for plotting long-term capacity decay.
--
-- One row per calendar day: the sampler upserts today's row rather than
-- appending, so a machine left running does not accumulate thousands of rows
-- and the table stays small enough to read in full for a chart.
--
-- max_capacity / design_capacity are the raw mAh values, kept instead of a
-- precomputed percentage so the health formula can change later without
-- invalidating history.
CREATE TABLE IF NOT EXISTS battery_health_snapshots (
    -- Local calendar day as YYYY-MM-DD, which is also the upsert key.
    day             TEXT    PRIMARY KEY NOT NULL,
    -- Unix seconds of the most recent sample written for this day.
    timestamp       INTEGER NOT NULL,
    max_capacity    INTEGER NOT NULL,
    design_capacity INTEGER NOT NULL,
    cycle_count     INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_battery_health_timestamp
    ON battery_health_snapshots (timestamp);
