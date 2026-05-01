-- Add migration script here
alter table water_config
drop column circulation_mode,
drop column circulation_on_sec,
drop column circulation_off_sec;
