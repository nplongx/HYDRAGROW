ALTER TABLE device_ownership
  ADD COLUMN mqtt_username TEXT,
  ADD COLUMN mqtt_password_hash TEXT; -- store the hash, never the plaintext password
