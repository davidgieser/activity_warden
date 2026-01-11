DELETE FROM timers
WHERE display_name = ?1
  AND host = ?2
  AND time_limit = ?3
  AND active_days = ?4;