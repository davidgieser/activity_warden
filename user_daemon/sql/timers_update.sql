UPDATE timers
SET
    time_limit = ?3,
    active_days = ?4
WHERE
    display_name = ?1
    AND host = ?2;