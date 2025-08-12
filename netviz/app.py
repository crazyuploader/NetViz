"""A Flask application for visualizing network data from PeeringDB."""

from collections import Counter
import os
from flask import Flask, render_template, jsonify, request
from werkzeug.middleware.proxy_fix import ProxyFix
from netviz.data import load_network_data

basedir = os.path.abspath(os.path.dirname(__file__))
template_dir = os.path.join(basedir, "..", "templates")

app = Flask(__name__, template_folder=template_dir)

# Enable reverse proxy support
app.wsgi_app = ProxyFix(app.wsgi_app, x_for=1, x_proto=1)

app.DATA = load_network_data()


@app.route("/")
def index():
    """Main dashboard with overview statistics"""
    total_networks = len(app.DATA)
    network_types = Counter(
        [item["info_type"] for item in app.DATA if item.get("info_type")]
    )
    policy_types = Counter(
        [item["policy_general"] for item in app.DATA if item.get("policy_general")]
    )
    scopes = Counter(
        [item["info_scope"] for item in app.DATA if item.get("info_scope")]
    )

    stats = {
        "total_networks": total_networks,
        "network_types": dict(network_types),
        "policy_types": dict(policy_types),
        "scopes": dict(scopes),
    }

    return render_template("dashboard.html", stats=stats, networks=app.DATA[:10])


@app.route("/api/network-types")
def api_network_types():
    """API endpoint for network types chart data"""
    network_types = Counter(
        [item["info_type"] for item in app.DATA if item.get("info_type")]
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

    for item in app.DATA:
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
    for item in app.DATA:
        if item.get("ix_count") is not None and item.get("fac_count") is not None:
            data.append(
                {"x": item["ix_count"], "y": item["fac_count"], "label": item["name"]}
            )
    return jsonify(data)


@app.route("/networks")
def networks_list():
    """Detailed networks listing page with pagination"""
    page = request.args.get("page", 1, type=int)
    per_page = request.args.get("per_page", 25, type=int)

    total_networks = len(app.DATA)
    total_pages = (total_networks + per_page - 1) // per_page

    start_index = (page - 1) * per_page
    end_index = start_index + per_page

    paginated_networks = app.DATA[start_index:end_index]

    return render_template(
        "networks.html",
        networks=paginated_networks,
        page=page,
        per_page=per_page,
        total_pages=total_pages,
        total_networks=total_networks,
    )


@app.route("/analytics")
def analytics():
    """Advanced analytics page"""
    return render_template("analytics.html")


@app.route("/search")
def search_networks():
    """Search networks by AS number or name"""
    query_asn = request.args.get("asn", type=int)
    query_name = request.args.get("name", type=str)

    results = []
    if query_asn is not None or query_name:
        for network in app.DATA:
            match_asn = False
            if query_asn is not None and network.get("asn") == query_asn:
                match_asn = True

            match_name = False
            if query_name and network.get("name"):
                if query_name.lower() in network["name"].lower():
                    match_name = True

            if (query_asn is not None and match_asn) or (query_name and match_name):
                results.append(network)

    return render_template(
        "search.html", results=results, query_asn=query_asn, query_name=query_name
    )
