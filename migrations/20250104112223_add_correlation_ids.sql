alter table planning_trace
    add column correlation_id text;

alter table thing_command
    add column correlation_id text;

alter table user_trigger
    add column correlation_id text;

