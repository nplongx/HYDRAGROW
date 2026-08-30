#!/bin/bash
set -e

# Start postgres in background using a different image if there are overlayfs issues or just use the local instance if we can get it up.
# We will use the system service since it seems docker overlayfs is broken.
sudo service postgresql start || true
export DATABASE_URL=postgres://postgres:postgres@localhost:5432/postgres
cd hydragrow-backend
cargo test get_device_dosing_reports_in_range -- --test-threads=1
