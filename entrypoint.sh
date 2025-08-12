#!/bin/bash

# Fetch PeeringDB data
uv run python netviz/fetch_peeringdb.py

# Start Gunicorn
exec uv run gunicorn --config gunicorn.conf.py main:app
