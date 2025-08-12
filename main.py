#!/usr/bin/env python3
"""NetViz: Main entry point for the Flask application."""
from netviz.app import app
from netviz.data import load_network_data

print("Loading network data...")
app.DATA = load_network_data()

if not app.DATA:
    print("No data loaded. Exiting.")
