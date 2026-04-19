#!/usr/bin/env python3
"""
Demo MCP Server for MPL testing.

A simple HTTP server that echoes requests and provides basic endpoints
for testing the MPL proxy.
"""

import json
import uuid
from datetime import datetime
from http.server import HTTPServer, BaseHTTPRequestHandler
from typing import Any, Dict


class DemoHandler(BaseHTTPRequestHandler):
    """Simple request handler for demo purposes."""

    # In-memory storage for demo
    events: Dict[str, Dict[str, Any]] = {}
    messages: list = []

    def do_GET(self):
        """Handle GET requests."""
        if self.path == "/health":
            self._send_json({"status": "healthy", "service": "demo-mcp-server"})
        elif self.path == "/capabilities":
            self._send_json({
                "name": "demo-mcp-server",
                "version": "1.0.0",
                "tools": [
                    {"name": "calendar.create", "stype": "org.calendar.Event.v1"},
                    {"name": "calendar.list", "stype": "org.calendar.Event.v1"},
                    {"name": "message.send", "stype": "org.communication.Message.v1"},
                    {"name": "echo", "stype": None},
                ],
                "stypes": [
                    "org.calendar.Event.v1",
                    "org.communication.Message.v1",
                    "org.agent.TaskPlan.v1",
                ]
            })
        elif self.path == "/api/events":
            self._send_json({"events": list(self.events.values())})
        elif self.path.startswith("/api/events/"):
            event_id = self.path.split("/")[-1]
            if event_id in self.events:
                self._send_json(self.events[event_id])
            else:
                self._send_error(404, "Event not found")
        else:
            self._send_json({"message": "Demo MCP Server", "path": self.path})

    def do_POST(self):
        """Handle POST requests."""
        content_length = int(self.headers.get("Content-Length", 0))
        body = self.rfile.read(content_length).decode("utf-8") if content_length > 0 else "{}"

        try:
            data = json.loads(body) if body else {}
        except json.JSONDecodeError:
            self._send_error(400, "Invalid JSON")
            return

        # Echo the MPL headers back
        mpl_headers = {
            k: v for k, v in self.headers.items()
            if k.lower().startswith("x-mpl")
        }

        if self.path == "/api/events":
            # Create a new event
            event_id = data.get("eventId") or f"evt-{uuid.uuid4().hex[:8]}"
            event = {
                "eventId": event_id,
                "title": data.get("title", "Untitled"),
                "start": data.get("start"),
                "end": data.get("end"),
                "description": data.get("description"),
                "location": data.get("location"),
                "attendees": data.get("attendees", []),
                "created_at": datetime.utcnow().isoformat() + "Z",
            }
            self.events[event_id] = event
            self._send_json(event, status=201, extra_headers=mpl_headers)

        elif self.path == "/api/messages":
            # Store a message
            message = {
                "id": f"msg-{uuid.uuid4().hex[:8]}",
                "content": data.get("content"),
                "sender": data.get("sender"),
                "recipient": data.get("recipient"),
                "timestamp": datetime.utcnow().isoformat() + "Z",
            }
            self.messages.append(message)
            self._send_json(message, status=201, extra_headers=mpl_headers)

        elif self.path == "/api/echo":
            # Echo back the request
            self._send_json({
                "echo": data,
                "headers": dict(self.headers),
                "mpl_headers": mpl_headers,
                "timestamp": datetime.utcnow().isoformat() + "Z",
            }, extra_headers=mpl_headers)

        elif self.path == "/api/tasks":
            # Create a task plan
            task = {
                "taskId": f"task-{uuid.uuid4().hex[:8]}",
                "goal": data.get("goal"),
                "steps": data.get("steps", []),
                "status": "created",
                "created_at": datetime.utcnow().isoformat() + "Z",
            }
            self._send_json(task, status=201, extra_headers=mpl_headers)

        else:
            # Default: echo the request
            self._send_json({
                "path": self.path,
                "method": "POST",
                "body": data,
                "mpl_headers": mpl_headers,
            }, extra_headers=mpl_headers)

    def _send_json(self, data: Dict[str, Any], status: int = 200, extra_headers: Dict[str, str] = None):
        """Send a JSON response."""
        body = json.dumps(data, indent=2).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", len(body))
        if extra_headers:
            for key, value in extra_headers.items():
                self.send_header(key, value)
        self.end_headers()
        self.wfile.write(body)

    def _send_error(self, status: int, message: str):
        """Send an error response."""
        self._send_json({"error": message}, status=status)

    def log_message(self, format, *args):
        """Log HTTP requests."""
        print(f"[{datetime.now().isoformat()}] {args[0]}")


def main():
    """Run the demo server."""
    port = 8080
    server = HTTPServer(("0.0.0.0", port), DemoHandler)
    print(f"Demo MCP Server running on http://0.0.0.0:{port}")
    print("Endpoints:")
    print("  GET  /health       - Health check")
    print("  GET  /capabilities - Server capabilities")
    print("  GET  /api/events   - List events")
    print("  POST /api/events   - Create event")
    print("  POST /api/messages - Send message")
    print("  POST /api/tasks    - Create task")
    print("  POST /api/echo     - Echo request")
    server.serve_forever()


if __name__ == "__main__":
    main()
