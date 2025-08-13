#!/bin/bash

# Exit immediately if a command exits with a non-zero status.
set -e
# Fail a pipeline if any command fails.
set -o pipefail

# Fetch PeeringDB data before starting the application.
uv run python netviz/fetch_peeringdb.py

# Start the Gunicorn server.
# 'exec' is used to replace the shell process with the Gunicorn process.
exec uv run gunicorn --config gunicorn.conf.py netviz.app:app
