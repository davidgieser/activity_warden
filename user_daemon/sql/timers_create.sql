CREATE TABLE timers (
    display_name VARCHAR(64) NOT NULL,
    host VARCHAR(64) NOT NULL,
    time_limit BIGINT NOT NULL,
    active_days TINYINT UNSIGNED NOT NULL,
    PRIMARY KEY (display_name, host)
);