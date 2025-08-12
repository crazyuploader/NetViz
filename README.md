# NetViz

> Visualize PeeringDB Data

## Overview

NetViz is a project aimed at visualizing data from PeeringDB.

## Usage

### Fetching PeeringDB Data

The `fetch_peeringdb_data.py` script is used to download JSON data from the PeeringDB API. The data will be saved in the `data/peeringdb/` directory.

To run the data fetching script:

```bash
python3 fetch_peeringdb_data.py
```

**API Key (Optional):**

If you have a PeeringDB API key, you can set it as an environment variable named `PEERINGDB_API_KEY` before running the script for authenticated access (which may provide higher rate limits):

```bash
export PEERINGDB_API_KEY="your_api_key_here"
python3 fetch_peeringdb_data.py
```

### Main Application

The `main.py` script is the primary entry point for the NetViz application. Currently, it's a placeholder, but it will eventually contain the logic for data visualization.

To run the main application:

```bash
python3 main.py
```

## Tools/Technologies

- [requests](https://pypi.org/project/requests/)
