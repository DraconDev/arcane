import os
import sys
import time
from http.server import HTTPServer, BaseHTTPRequestHandler

PORT = int(os.environ.get("PORT", 8080))

class Handler(BaseHTTPRequestHandler):
    def do_GET(self):
        print(f"[{time.ctime()}] 📥 Request: {self.path}", file=sys.stdout)
        sys.stdout.flush()

        if self.path == '/health':
            self.send_response(200)
            self.end_headers()
            self.wfile.write(b"OK")
            return

        if self.path == '/crash':
            print("💥 CRASHING NOW...", file=sys.stderr)
            sys.exit(1)

        self.send_response(200)
        self.end_headers()
        body = f"🚀 Hello from Arcane!\nHostname: {os.environ.get('HOSTNAME', 'unknown')}\nSecret: {os.environ.get('SECRET_KEY', 'NOT_SET')}\n"
        self.wfile.write(body.encode())

print(f"🔥 Arcane Feature Test App starting on port {PORT}...", file=sys.stdout)
sys.stdout.flush()
HTTPServer(('0.0.0.0', PORT), Handler).serve_forever()
