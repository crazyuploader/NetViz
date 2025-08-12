#
# Created by Jugal Kishore -- 2025
#
# Using Python 3.13
FROM python:3.13.6-slim

# Set Time Zone to IST
ENV TZ="Asia/Kolkata"
ENV FLASK_ENV=production

# Set Working Directory
WORKDIR /app

# Copy File(s)
COPY . /app

# Make entrypoint.sh executable
RUN chmod +x /app/entrypoint.sh

# Installing Package(s)
RUN pip3 install --upgrade pip

COPY --from=ghcr.io/astral-sh/uv:latest /uv /uvx /bin/

# Install Dependencies
RUN uv sync --frozen --no-install-project --no-dev --python-preference=only-system

# Expose Port
EXPOSE 8201

CMD ["/app/entrypoint.sh"]
