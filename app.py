"""A Flask application for visualizing network data from PeeringDB.
"""
from collections import Counter
import json
from flask import Flask, render_template, jsonify


app = Flask(__name__)

DATA = {}

try:
    with open("data/peeringdb/net.json", "r", encoding="utf-8") as file:
        content = file.read()
        DATA = json.loads(content)["data"]
except FileNotFoundError:
    print("Error: The file 'data/peeringdb/networks.json' was not found.")
except json.JSONDecodeError as e:
    print(f"An error occurred: {e}")


@app.route("/")
def index():
    """Main dashboard with overview statistics"""
    total_networks = len(DATA)
    network_types = Counter(
        [item["info_type"] for item in DATA if item.get("info_type")]
    )
    policy_types = Counter(
        [item["policy_general"] for item in DATA if item.get("policy_general")]
    )
    scopes = Counter([item["info_scope"] for item in DATA if item.get("info_scope")])

    stats = {
        "total_networks": total_networks,
        "network_types": dict(network_types),
        "policy_types": dict(policy_types),
        "scopes": dict(scopes),
    }

    return render_template("dashboard.html", stats=stats, networks=DATA[:10])


@app.route("/api/network-types")
def api_network_types():
    """API endpoint for network types chart data"""
    network_types = Counter(
        [item["info_type"] for item in DATA if item.get("info_type")]
    )
    return jsonify(
        {"labels": list(network_types.keys()), "data": list(network_types.values())}
    )


@app.route("/api/prefixes-distribution")
def api_prefixes_distribution():
    """API endpoint for IPv4/IPv6 prefixes distribution"""
    networks = []
    ipv4_prefixes = []
    ipv6_prefixes = []

    for item in DATA:
        if item.get("info_prefixes4") and item.get("info_prefixes6"):
            networks.append(
                item["name"][:30] + "..." if len(item["name"]) > 30 else item["name"]
            )
            ipv4_prefixes.append(item["info_prefixes4"])
            ipv6_prefixes.append(item["info_prefixes6"])

    return jsonify(
        {
            "networks": networks[:15],  # Limit to top 15 for readability
            "ipv4": ipv4_prefixes[:15],
            "ipv6": ipv6_prefixes[:15],
        }
    )


@app.route("/api/ix-facility-correlation")
def api_ix_facility_correlation():
    """API endpoint for IX count vs Facility count correlation"""
    data = []
    for item in DATA:
        if item.get("ix_count") is not None and item.get("fac_count") is not None:
            data.append(
                {"x": item["ix_count"], "y": item["fac_count"], "label": item["name"]}
            )
    return jsonify(data)


@app.route("/networks")
def networks_list():
    """Detailed networks listing page"""
    return render_template("networks.html", networks=DATA)


@app.route("/analytics")
def analytics():
    """Advanced analytics page"""
    return render_template("analytics.html")


if __name__ == "__main__":
    app.run(debug=True)
