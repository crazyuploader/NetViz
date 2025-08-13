#!/usr/bin/env python3
"""NetViz: Main entry point for the Flask application."""
from netviz.fetch_peeringdb import fetch_and_save_peeringdb_data


def main():
    """Fetches latest network data, loads it, and starts the Flask web server."""
    print("Fetching latest network data from PeeringDB...")
    fetch_and_save_peeringdb_data()


if __name__ == "__main__":
    main()
