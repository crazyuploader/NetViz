#!/usr/bin/env python3
"""Fetches and saves JSON data from the PeeringDB API."""
import json
import os
from pathlib import Path
import requests


# Configuration
BASE_API_URL = "https://www.peeringdb.com/api/"
OUTPUT_DIR = Path("data/peeringdb")
PEERINGDB_API_KEY = os.getenv("PEERINGDB_API_KEY", "")


def fetch_and_save_peeringdb_data():
    """Fetches and saves JSON data from PeeringDB API endpoints."""
    # Create the output directory if it doesn't exist.
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

    print(f"Fetching API index from {BASE_API_URL}...")
    try:
        response = requests.get(BASE_API_URL, timeout=30)
        response.raise_for_status()  # Raise an exception for HTTP errors (e.g., 404, 500).
        api_index = response.json()
    except requests.exceptions.RequestException as e:
        print(f"Error fetching API index: {e}")
        return  # Exit the function if the API index cannot be fetched.

    endpoints_data = api_index.get("data")
    if not isinstance(endpoints_data, list) or not endpoints_data:
        print("Error: 'data' key in API index is not a non-empty list.")
        return
    endpoints = endpoints_data[0]
    if not isinstance(endpoints, dict):
        print("Error: First element of 'data' is not a dictionary of endpoints.")
        return

    headers = {}
    if PEERINGDB_API_KEY:
        print("API Key for PeeringDB found, using it.")
        headers = {"Authorization": "Api-Key " + PEERINGDB_API_KEY}

    for name, url in endpoints.items():
        file_path = OUTPUT_DIR / f"{name}.json"

        print(f"Fetching data for '{name}' from {url}...")
        try:
            response = requests.get(url, timeout=30, headers=headers)
            response.raise_for_status()
            data = response.json()

            with open(file_path, "w", encoding="utf-8") as f:
                json.dump(data, f, indent=4)
            print(f"Successfully saved data to {file_path}")
        except requests.exceptions.RequestException as e:
            print(f"Error fetching data from {url}: {e}")
        except IOError as e:
            print(f"Error saving data to {file_path}: {e}")
        except json.JSONDecodeError:
            print(f"Error decoding JSON from {url}. Response was not valid JSON.")


if __name__ == "__main__":
    fetch_and_save_peeringdb_data()
