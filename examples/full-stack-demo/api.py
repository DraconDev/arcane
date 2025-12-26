"""
Full-Stack Demo API
A simple Flask API that demonstrates environment variable usage.
"""
import os
import json
import time
from flask import Flask, jsonify, request

app = Flask(__name__)

@app.route('/health')
def health():
    return 'OK', 200

@app.route('/')
def index():
    return jsonify({
        "service": "Full-Stack Demo API",
        "version": "1.0.0",
        "environment": os.environ.get("APP_ENV", "unknown"),
        "timestamp": time.time()
    })

@app.route('/env')
def show_env():
    """Show masked environment variables (for demo only)"""
    safe_env = {}
    for key, value in os.environ.items():
        if any(secret in key.upper() for secret in ['KEY', 'SECRET', 'PASSWORD', 'TOKEN']):
            safe_env[key] = value[:4] + '****' if len(value) > 4 else '****'
        else:
            safe_env[key] = value
    return jsonify(safe_env)

@app.route('/db-check')
def db_check():
    """Check database connectivity"""
    db_url = os.environ.get('DATABASE_URL', 'not configured')
    # In real app, we'd actually connect
    return jsonify({"database": "configured" if 'postgres' in db_url else "not configured"})

@app.route('/cache-check')
def cache_check():
    """Check Redis connectivity"""
    redis_url = os.environ.get('REDIS_URL', 'not configured')
    return jsonify({"redis": "configured" if 'redis' in redis_url else "not configured"})

@app.route('/queue-check')
def queue_check():
    """Check RabbitMQ connectivity"""
    mq_url = os.environ.get('RABBITMQ_URL', 'not configured')
    return jsonify({"rabbitmq": "configured" if 'amqp' in mq_url else "not configured"})

@app.route('/services')
def services():
    """List all configured services"""
    return jsonify({
        "database": bool(os.environ.get('DATABASE_URL')),
        "redis": bool(os.environ.get('REDIS_URL')),
        "rabbitmq": bool(os.environ.get('RABBITMQ_URL')),
        "stripe": bool(os.environ.get('STRIPE_SECRET_KEY')),
        "sendgrid": bool(os.environ.get('SENDGRID_API_KEY')),
        "twilio": bool(os.environ.get('TWILIO_ACCOUNT_SID')),
        "aws": bool(os.environ.get('AWS_ACCESS_KEY_ID')),
        "google_oauth": bool(os.environ.get('GOOGLE_CLIENT_ID')),
        "github_oauth": bool(os.environ.get('GITHUB_CLIENT_ID')),
        "sentry": bool(os.environ.get('SENTRY_DSN')),
        "openai": bool(os.environ.get('OPENAI_API_KEY')),
    })

if __name__ == '__main__':
    app.run(host='0.0.0.0', port=8080, debug=True)
