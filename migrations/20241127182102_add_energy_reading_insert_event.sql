CREATE OR REPLACE FUNCTION notify_energy_reading() RETURNS TRIGGER AS $$
BEGIN
    PERFORM pg_notify(
        TG_ARGV[0],
        json_build_object(
            'id', NEW.id
        )::text
    );
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;


-- Trigger for INSERT
CREATE TRIGGER energy_reading_insert_notify
AFTER INSERT ON energy_reading
FOR EACH ROW EXECUTE FUNCTION notify_energy_reading('energy_reading_insert');

