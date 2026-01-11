CREATE TABLE focus_changes (
    display_name VARCHAR(64) NOT NULL,
    host VARCHAR(64) NOT NULL,
    ts TIMESTAMP(6) NOT NULL,
    duration_seconds INT UNSIGNED NOT NULL,
    PRIMARY KEY (display_name, host, ts)
);