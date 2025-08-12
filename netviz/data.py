"""Data loading and processing for network visualization."""

import json


def load_network_data():
    """Loads network data from the JSON file."""
    data = {}
    try:
        with open("data/peeringdb/net.json", "r", encoding="utf-8") as file:
            content = file.read()
            data = json.loads(content)["data"]
    except FileNotFoundError:
        print("Error: The file 'data/peeringdb/net.json' was not found.")
    except json.JSONDecodeError as e:
        print(f"An error occurred while decoding JSON: {e}")
    return data
