-- Add migration script here
alter table dosing_calibration
drop column tank_volume_l;
