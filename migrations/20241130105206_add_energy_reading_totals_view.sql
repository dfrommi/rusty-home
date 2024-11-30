-- heating readings are reset to zero at the end of the year
CREATE OR REPLACE VIEW energy_reading_total AS
(
  SELECT
      id,
      type,
      name,
      timestamp,
      value + coalesce(
          (SELECT SUM(value) 
          FROM energy_reading er2 
          WHERE er2.type = er1.type
            AND er2.name = er1.name
            AND er2.year_end = true
            AND er2.timestamp < er1.timestamp
    ), 0) AS value
  FROM 
      energy_reading er1
  WHERE type = 'heating'
)
UNION
(
	SELECT
		ID,
		TYPE,
		NAME,
		TIMESTAMP,
		VALUE
	FROM
		ENERGY_READING
	WHERE
		TYPE != 'heating'
)
