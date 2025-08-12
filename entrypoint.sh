#!/bin/bash

# Fetch PeeringDB data
python uv run netviz/fetch_peeringdb.py

# Start Gunicorn
exec uv run gunicorn --workers 4 --bind 0.0.0.0:8201 main:app --access-logfile - --error-logfile -
