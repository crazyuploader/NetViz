#
# Created by Jugal Kishore -- 2025
#
# Using Python 3.13
FROM python:3.13.6-slim

# Set Time Zone to IST
ENV TZ="Asia/Kolkata"
ENV FLASK_ENV=production

# Add required apt packages
RUN apt-get update && \
    apt-get install --yes --no-install-recommends \
    curl ca-certificates wget && \
    rm -rf /var/lib/apt/lists/* /tmp/*

# Set Working Directory
WORKDIR /app

# Copy File(s)
COPY . /app

# Make entrypoint.sh executable
RUN chmod +x /app/entrypoint.sh

# Installing Package(s)
RUN pip3 install --upgrade pip

# Copy uv binaries from its Docker Image
COPY --from=ghcr.io/astral-sh/uv:latest /uv /uvx /usr/local/bin/

# Add user
RUN groupadd --system netvizgroup && useradd --system --gid netvizgroup netvizuser --create-home

# Set ownership of /app to the new user
RUN chown -R netvizuser:netvizgroup /app

# Switch to the non-root user
USER netvizuser 

# Install Dependencies
RUN uv sync --frozen --no-install-project --no-dev --python-preference=only-system

# Expose Port
EXPOSE 8201

CMD ["/app/entrypoint.sh"]
