#!/usr/bin/env python3
"""
Program to fetch and load JSON data from PeeringDB API.
This script connects to the PeeringDB API, retrieves a list of available data endpoints,
and then fetches data from each endpoint, saving it as a JSON file in a specified directory.
It supports using an API key for authenticated access and skips fetching data if the
corresponding file already exists locally.
"""
import json
import os
from pathlib import Path
import requests


# --- Configuration Variables ---
# Base URL for the PeeringDB API. All API requests will be made relative to this URL.
BASE_API_URL = "https://www.peeringdb.com/api/"
# Directory where the fetched JSON data will be saved.
# This path is relative to the script's execution directory.
OUTPUT_DIR = Path("data/peeringdb")
# PeeringDB API key, retrieved from environment variables for security.
# If not set, requests will be unauthenticated.
PEERINGDB_API_KEY = os.getenv("PEERINGDB_API_KEY", "")


def fetch_and_save_peeringdb_data():
    """
    Fetches JSON data from PeeringDB API endpoints and saves them to
    data/peeringdb/ directory. Skips fetching if the file already exists.
    """
    # Create the output directory if it doesn't exist.
    # `parents=True` ensures any necessary parent directories are also created.
    # `exist_ok=True` prevents an error if the directory already exists.
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

    # Fetch the initial API index to get all available endpoints.
    # This index provides the names and URLs for various data sets (e.g., 'ix', 'fac').
    print(f"Fetching API index from {BASE_API_URL}...")
    try:
        response = requests.get(BASE_API_URL, timeout=30)
        response.raise_for_status()  # Raise an exception for HTTP errors (e.g., 404, 500).
        api_index = response.json()
    except requests.exceptions.RequestException as e:
        print(f"Error fetching API index: {e}")
        return  # Exit the function if the API index cannot be fetched.

    # Extract endpoints from the 'data' key. The API index is expected to have a 'data' key
    # which contains a list, and the first element of that list is a dictionary
    # mapping endpoint names to their URLs.
    endpoints_data = api_index.get("data")
    if not isinstance(endpoints_data, list) or not endpoints_data:
        print("Error: 'data' key in API index is not a non-empty list.")
        return
    endpoints = endpoints_data[0]
    if not isinstance(endpoints, dict):
        print("Error: First element of 'data' is not a dictionary of endpoints.")
        return

    # Set headers for the requests, including API key if available.
    # An API key can provide higher rate limits or access to more data.
    headers = {}
    if PEERINGDB_API_KEY:
        print("API Key for PeeringDB found, using it.")
        headers = {"Authorization": "Api-Key " + PEERINGDB_API_KEY}

    # Iterate through each endpoint, fetch data, and save it.
    for name, url in endpoints.items():
        file_path = OUTPUT_DIR / f"{name}.json"

        print(f"Fetching data for '{name}' from {url}...")
        try:
            # Make the HTTP GET request to the endpoint.
            # A timeout is set to prevent the request from hanging indefinitely.
            response = requests.get(url, timeout=30, headers=headers)
            response.raise_for_status()  # Check for HTTP errors.
            data = response.json()  # Parse the JSON response.

            # Save the fetched data to a JSON file.
            # `indent=4` makes the JSON output human-readable.
            with open(file_path, "w", encoding="utf-8") as f:
                json.dump(data, f, indent=4)
            print(f"Successfully saved data to {file_path}")
        except requests.exceptions.RequestException as e:
            # Catch errors related to the HTTP request itself (e.g., network issues, bad URL).
            print(f"Error fetching data from {url}: {e}")
        except IOError as e:
            # Catch errors related to file operations (e.g., permission denied, disk full).
            print(f"Error saving data to {file_path}: {e}")
        except json.JSONDecodeError:
            # Catch errors if the response content is not valid JSON.
            print(f"Error decoding JSON from {url}. Response was not valid JSON.")


if __name__ == "__main__":
    # This ensures that fetch_and_save_peeringdb_data() is called only when the script
    # is executed directly (not when imported as a module).
    fetch_and_save_peeringdb_data()
