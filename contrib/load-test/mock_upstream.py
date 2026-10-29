"""Upstream stand-in for load tests: answers every POST at once with the
same chat completion, so the proxy's own cost is all that is measured.

    python3 contrib/load-test/mock_upstream.py [PORT]   (default 9099)
"""

import json
import sys
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

BODY = json.dumps(
    {
        "id": "chatcmpl-load-test",
        "object": "chat.completion",
        "created": 0,
        "model": "load-test",
        "choices": [
            {
                "index": 0,
                "message": {"role": "assistant", "content": "Here is the summary."},
                "finish_reason": "stop",
            }
        ],
        "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2},
    }
).encode()


class Handler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def do_POST(self):
        self.rfile.read(int(self.headers.get("content-length", 0)))
        self.send_response(200)
        self.send_header("content-type", "application/json")
        self.send_header("content-length", str(len(BODY)))
        self.end_headers()
        self.wfile.write(BODY)

    def log_message(self, *args):
        pass


if __name__ == "__main__":
    port = int(sys.argv[1]) if len(sys.argv) > 1 else 9099
    ThreadingHTTPServer(("127.0.0.1", port), Handler).serve_forever()
