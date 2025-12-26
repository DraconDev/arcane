"""
Full-Stack Demo Worker
A simple background worker that processes jobs from RabbitMQ.
"""
import os
import time
import sys

print("🔧 Starting Background Worker...")
print(f"   Environment: {os.environ.get('APP_ENV', 'unknown')}")
print(f"   Database: {'✅' if os.environ.get('DATABASE_URL') else '❌'}")
print(f"   Redis: {'✅' if os.environ.get('REDIS_URL') else '❌'}")
print(f"   RabbitMQ: {'✅' if os.environ.get('RABBITMQ_URL') else '❌'}")
sys.stdout.flush()

# Simulate worker loop
while True:
    print(f"[{time.strftime('%Y-%m-%d %H:%M:%S')}] 💤 Waiting for jobs...")
    sys.stdout.flush()
    time.sleep(30)
    print(f"[{time.strftime('%Y-%m-%d %H:%M:%S')}] ✅ Heartbeat OK")
    sys.stdout.flush()
