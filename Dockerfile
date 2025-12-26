# Simple verification app
FROM python:3.9-slim

# Install dependencies? No, just built-in http.server for now.
WORKDIR /app

# Copy this repo into the container
COPY . /app

# Expose the standard Arcane health port
EXPOSE 3000

# Run a simple HTTP server on port 3000
CMD ["python3", "-m", "http.server", "3000"]
