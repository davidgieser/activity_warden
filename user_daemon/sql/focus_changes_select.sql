SELECT
  display_name,
  host,
  SUM(duration_seconds) AS total_duration
FROM focus_changes
WHERE ts >= ?1
  AND ts < ?2
GROUP BY display_name, host;
