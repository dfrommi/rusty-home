CREATE OR REPLACE FUNCTION notify_thing_values_insert() RETURNS TRIGGER AS $$
BEGIN
    PERFORM pg_notify(
        'thing_values_insert', 
        json_build_object(
            'id', NEW.id,
            'tag_id', NEW.tag_id
        )::text
    );
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER thing_values_insert_notify
AFTER INSERT ON THING_VALUES
FOR EACH ROW EXECUTE FUNCTION notify_thing_values_insert();


