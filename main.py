#!/usr/bin/env python3
"""NetViz: Main entry point for the Flask application."""
import os
from netviz.app import app
from netviz.data import load_network_data


def main():
    """Loads network data and starts the Flask web server."""
    print("Loading network data...")
    app.DATA = load_network_data()

    if not app.DATA:
        print("No data loaded. Exiting.")
        return

    print("Starting Flask application...")
    # Determine if running in production mode
    is_production = os.environ.get("FLASK_ENV") == "production"

    # Run the Flask application
    app.run(debug=not is_production, port=8201)


if __name__ == "__main__":
    main()
