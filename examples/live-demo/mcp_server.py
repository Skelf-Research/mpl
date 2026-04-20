#!/usr/bin/env python3
"""
Enhanced MCP Demo Server for MPL Live Demo

A more realistic MCP server with multiple tools and proper responses
for demonstrating MPL features.
"""

import json
import uuid
from datetime import datetime, timedelta
from http.server import HTTPServer, BaseHTTPRequestHandler
from typing import Any, Dict, List, Optional
from urllib.parse import urlparse, parse_qs


class InMemoryStore:
    """Simple in-memory storage for demo data."""

    def __init__(self):
        self.events: Dict[str, Dict] = {}
        self.messages: List[Dict] = []
        self.tasks: Dict[str, Dict] = {}
        self.notifications: List[Dict] = []

    def reset(self):
        """Clear all data."""
        self.events.clear()
        self.messages.clear()
        self.tasks.clear()
        self.notifications.clear()


store = InMemoryStore()


class MCPHandler(BaseHTTPRequestHandler):
    """Enhanced MCP request handler with rich tool implementations."""

    # Protocol version
    PROTOCOL_VERSION = "mcp/1.1"
    MPL_VERSION = "0.1"

    def do_GET(self):
        """Handle GET requests."""
        parsed = urlparse(self.path)
        path = parsed.path
        query = parse_qs(parsed.query)

        if path == "/health":
            self._send_json({
                "status": "healthy",
                "service": "mcp-demo-server",
                "version": "2.0.0",
                "uptime_seconds": 12345,
            })

        elif path == "/capabilities":
            self._send_capabilities()

        elif path == "/.well-known/ai-alpn":
            # AI-ALPN discovery endpoint
            self._send_json({
                "protocols": [self.PROTOCOL_VERSION],
                "mpl_version": self.MPL_VERSION,
                "stypes": self._get_supported_stypes(),
                "tools": self._get_available_tools(),
                "profiles": ["qom-basic", "qom-strict-argcheck"],
            })

        elif path == "/api/events":
            events = list(store.events.values())
            self._send_json({
                "events": events,
                "count": len(events),
            })

        elif path.startswith("/api/events/"):
            event_id = path.split("/")[-1]
            if event_id in store.events:
                self._send_json(store.events[event_id])
            else:
                self._send_error(404, f"Event not found: {event_id}")

        elif path == "/api/tasks":
            tasks = list(store.tasks.values())
            self._send_json({
                "tasks": tasks,
                "count": len(tasks),
            })

        elif path.startswith("/api/tasks/"):
            task_id = path.split("/")[-1]
            if task_id in store.tasks:
                self._send_json(store.tasks[task_id])
            else:
                self._send_error(404, f"Task not found: {task_id}")

        elif path == "/api/messages":
            self._send_json({
                "messages": store.messages[-50:],  # Last 50 messages
                "count": len(store.messages),
            })

        elif path == "/_mpl/qom":
            # QoM metrics endpoint
            self._send_json({
                "requests_total": 42,
                "validations_passed": 38,
                "validations_failed": 4,
                "avg_schema_fidelity": 0.95,
                "profiles": {
                    "qom-basic": {"requests": 30, "pass_rate": 0.97},
                    "qom-strict-argcheck": {"requests": 12, "pass_rate": 0.85},
                },
            })

        else:
            self._send_json({
                "message": "MCP Demo Server",
                "path": path,
                "query": query,
                "hint": "Try /capabilities or /health",
            })

    def do_POST(self):
        """Handle POST requests."""
        content_length = int(self.headers.get("Content-Length", 0))
        body = self.rfile.read(content_length).decode("utf-8") if content_length > 0 else "{}"

        try:
            data = json.loads(body) if body else {}
        except json.JSONDecodeError as e:
            self._send_error(400, f"Invalid JSON: {e}")
            return

        # Extract MPL headers
        mpl_headers = self._extract_mpl_headers()

        parsed = urlparse(self.path)
        path = parsed.path

        if path == "/api/events":
            self._handle_create_event(data, mpl_headers)

        elif path == "/api/events/query":
            self._handle_query_events(data, mpl_headers)

        elif path == "/api/messages":
            self._handle_send_message(data, mpl_headers)

        elif path == "/api/tasks":
            self._handle_create_task(data, mpl_headers)

        elif path.startswith("/api/tasks/") and path.endswith("/execute"):
            task_id = path.split("/")[-2]
            self._handle_execute_task(task_id, data, mpl_headers)

        elif path == "/api/tools/invoke":
            self._handle_tool_invocation(data, mpl_headers)

        elif path == "/api/notifications":
            self._handle_send_notification(data, mpl_headers)

        elif path == "/api/echo":
            self._send_json({
                "echo": data,
                "headers": dict(self.headers),
                "mpl_headers": mpl_headers,
                "timestamp": datetime.utcnow().isoformat() + "Z",
                "server": "mcp-demo-server",
            }, extra_headers=mpl_headers)

        elif path == "/.well-known/ai-alpn":
            # AI-ALPN handshake
            self._handle_handshake(data)

        elif path == "/api/reset":
            store.reset()
            self._send_json({"status": "reset", "message": "All data cleared"})

        else:
            # Default: echo with metadata
            self._send_json({
                "path": path,
                "method": "POST",
                "body": data,
                "mpl_headers": mpl_headers,
                "timestamp": datetime.utcnow().isoformat() + "Z",
            }, extra_headers=mpl_headers)

    def _handle_create_event(self, data: Dict, mpl_headers: Dict):
        """Create a calendar event."""
        event_id = data.get("eventId") or f"evt-{uuid.uuid4().hex[:8]}"

        # Validate required fields
        if "title" not in data:
            self._send_error(400, "Missing required field: title")
            return

        event = {
            "eventId": event_id,
            "title": data.get("title"),
            "start": data.get("start"),
            "end": data.get("end"),
            "description": data.get("description"),
            "location": data.get("location"),
            "attendees": data.get("attendees", []),
            "recurrence": data.get("recurrence"),
            "reminders": data.get("reminders", []),
            "status": "confirmed",
            "created_at": datetime.utcnow().isoformat() + "Z",
            "updated_at": datetime.utcnow().isoformat() + "Z",
        }

        store.events[event_id] = event
        self._send_json(event, status=201, extra_headers=mpl_headers)

    def _handle_query_events(self, data: Dict, mpl_headers: Dict):
        """Query calendar events."""
        # Simple query implementation
        start_after = data.get("start_after")
        end_before = data.get("end_before")
        limit = data.get("limit", 10)

        events = list(store.events.values())

        # Apply filters (simplified)
        if start_after:
            events = [e for e in events if e.get("start", "") >= start_after]
        if end_before:
            events = [e for e in events if e.get("end", "") <= end_before]

        # Apply limit
        events = events[:limit]

        self._send_json({
            "events": events,
            "count": len(events),
            "total": len(store.events),
            "query": data,
        }, extra_headers=mpl_headers)

    def _handle_send_message(self, data: Dict, mpl_headers: Dict):
        """Send a message."""
        message = {
            "id": data.get("id") or f"msg-{uuid.uuid4().hex[:8]}",
            "content": data.get("content"),
            "sender": data.get("sender"),
            "recipient": data.get("recipient"),
            "channel": data.get("channel", "default"),
            "metadata": data.get("metadata", {}),
            "timestamp": datetime.utcnow().isoformat() + "Z",
            "status": "sent",
        }

        store.messages.append(message)
        self._send_json(message, status=201, extra_headers=mpl_headers)

    def _handle_create_task(self, data: Dict, mpl_headers: Dict):
        """Create a task plan."""
        task_id = data.get("taskId") or data.get("planId") or f"task-{uuid.uuid4().hex[:8]}"

        task = {
            "taskId": task_id,
            "planId": task_id,
            "goal": data.get("goal"),
            "description": data.get("description"),
            "steps": data.get("steps", []),
            "status": data.get("status", "created"),
            "priority": data.get("priority", "normal"),
            "createdBy": data.get("createdBy"),
            "assignedTo": data.get("assignedTo"),
            "created_at": datetime.utcnow().isoformat() + "Z",
            "updated_at": datetime.utcnow().isoformat() + "Z",
        }

        store.tasks[task_id] = task
        self._send_json(task, status=201, extra_headers=mpl_headers)

    def _handle_execute_task(self, task_id: str, data: Dict, mpl_headers: Dict):
        """Execute a task step."""
        if task_id not in store.tasks:
            self._send_error(404, f"Task not found: {task_id}")
            return

        task = store.tasks[task_id]
        step_id = data.get("stepId")

        # Update step status
        for step in task.get("steps", []):
            if step.get("stepId") == step_id:
                step["status"] = "completed"
                step["completed_at"] = datetime.utcnow().isoformat() + "Z"
                break

        task["updated_at"] = datetime.utcnow().isoformat() + "Z"

        # Check if all steps are complete
        all_complete = all(
            s.get("status") == "completed"
            for s in task.get("steps", [])
        )
        if all_complete:
            task["status"] = "completed"

        self._send_json({
            "task": task,
            "executed_step": step_id,
            "all_complete": all_complete,
        }, extra_headers=mpl_headers)

    def _handle_tool_invocation(self, data: Dict, mpl_headers: Dict):
        """Handle a tool invocation."""
        tool_id = data.get("toolId")
        args = data.get("args", {})

        # Simulate different tool behaviors
        if tool_id == "calendar.query":
            result = {
                "availableSlots": [
                    {
                        "start": "2025-01-16T10:00:00Z",
                        "end": "2025-01-16T11:00:00Z",
                        "duration_minutes": 60,
                    },
                    {
                        "start": "2025-01-17T14:00:00Z",
                        "end": "2025-01-17T15:00:00Z",
                        "duration_minutes": 60,
                    },
                ],
                "timezone": "UTC",
            }

        elif tool_id == "calendar.create":
            event_id = f"evt-{uuid.uuid4().hex[:8]}"
            result = {
                "eventId": event_id,
                "title": args.get("title", "Meeting"),
                "start": args.get("start"),
                "end": args.get("end"),
                "created": True,
                "link": f"https://calendar.example.com/event/{event_id}",
            }

        elif tool_id == "notification.send":
            result = {
                "notificationId": f"notif-{uuid.uuid4().hex[:8]}",
                "sent": True,
                "recipients": args.get("recipients", []),
                "delivery_status": "delivered",
            }

        elif tool_id == "search.query":
            result = {
                "results": [
                    {"id": "doc-1", "title": "Result 1", "score": 0.95},
                    {"id": "doc-2", "title": "Result 2", "score": 0.87},
                ],
                "total": 2,
                "query": args.get("query", ""),
            }

        elif tool_id == "data.query":
            result = {
                "rows": [
                    {"id": 1, "value": "Sample data 1"},
                    {"id": 2, "value": "Sample data 2"},
                ],
                "columns": ["id", "value"],
                "count": 2,
            }

        else:
            result = {
                "toolId": tool_id,
                "args": args,
                "status": "executed",
                "message": f"Tool '{tool_id}' executed successfully",
            }

        response = {
            "resultId": f"res-{uuid.uuid4().hex[:8]}",
            "invocationId": data.get("invocationId"),
            "toolId": tool_id,
            "status": "success",
            "result": result,
            "timestamp": datetime.utcnow().isoformat() + "Z",
        }

        self._send_json(response, extra_headers=mpl_headers)

    def _handle_send_notification(self, data: Dict, mpl_headers: Dict):
        """Send a notification."""
        notification = {
            "id": f"notif-{uuid.uuid4().hex[:8]}",
            "type": data.get("type", "info"),
            "title": data.get("title"),
            "message": data.get("message"),
            "recipients": data.get("recipients", []),
            "priority": data.get("priority", "normal"),
            "sent_at": datetime.utcnow().isoformat() + "Z",
            "status": "delivered",
        }

        store.notifications.append(notification)
        self._send_json(notification, status=201, extra_headers=mpl_headers)

    def _handle_handshake(self, data: Dict):
        """Handle AI-ALPN handshake."""
        client_protocols = data.get("protocols", [])
        client_stypes = data.get("stypes", [])
        client_profile = data.get("profile", "qom-basic")

        # Negotiate protocol
        supported_protocols = [self.PROTOCOL_VERSION]
        selected_protocol = None
        for p in client_protocols:
            if p in supported_protocols:
                selected_protocol = p
                break

        if not selected_protocol:
            selected_protocol = self.PROTOCOL_VERSION

        # Negotiate STypes
        supported_stypes = self._get_supported_stypes()
        selected_stypes = [s for s in client_stypes if s in supported_stypes]

        # Track downgrades
        downgrades = []
        if client_profile == "qom-strict-argcheck":
            # Simulate partial support
            downgrades.append({
                "capability": "ext.qom.determinism@v1",
                "reason": "not supported",
            })

        response = {
            "success": True,
            "selected": {
                "protocol": selected_protocol,
                "mpl_version": self.MPL_VERSION,
                "stypes": selected_stypes or supported_stypes[:5],
                "tools": self._get_available_tools(),
                "profile": client_profile,
            },
            "downgrades": downgrades,
        }

        self._send_json(response)

    def _get_supported_stypes(self) -> List[str]:
        """Get list of supported semantic types."""
        return [
            "org.calendar.Event.v1",
            "org.communication.Message.v1",
            "org.agent.TaskPlan.v1",
            "org.agent.ToolInvocation.v1",
            "org.agent.ToolResult.v1",
            "org.notification.Alert.v1",
            "data.query.Query.v1",
            "data.record.Record.v1",
            "eval.rag.RAGQuery.v1",
            "eval.rag.RAGResponse.v1",
        ]

    def _get_available_tools(self) -> List[Dict]:
        """Get list of available tools."""
        return [
            {
                "id": "calendar.create",
                "name": "Create Calendar Event",
                "args_stype": "org.calendar.Event.v1",
                "result_stype": "org.calendar.Event.v1",
            },
            {
                "id": "calendar.query",
                "name": "Query Calendar",
                "args_stype": "data.query.Query.v1",
            },
            {
                "id": "message.send",
                "name": "Send Message",
                "args_stype": "org.communication.Message.v1",
            },
            {
                "id": "notification.send",
                "name": "Send Notification",
                "args_stype": "org.notification.Alert.v1",
            },
            {
                "id": "search.query",
                "name": "Search Documents",
                "args_stype": "data.query.Query.v1",
            },
        ]

    def _send_capabilities(self):
        """Send server capabilities."""
        self._send_json({
            "name": "mcp-demo-server",
            "version": "2.0.0",
            "description": "Enhanced MCP Demo Server for MPL testing",
            "protocols": [self.PROTOCOL_VERSION],
            "mpl": {
                "version": self.MPL_VERSION,
                "stypes": self._get_supported_stypes(),
                "profiles": ["qom-basic", "qom-strict-argcheck"],
            },
            "tools": self._get_available_tools(),
            "endpoints": [
                {"path": "/api/events", "methods": ["GET", "POST"]},
                {"path": "/api/messages", "methods": ["GET", "POST"]},
                {"path": "/api/tasks", "methods": ["GET", "POST"]},
                {"path": "/api/tools/invoke", "methods": ["POST"]},
                {"path": "/api/notifications", "methods": ["POST"]},
                {"path": "/.well-known/ai-alpn", "methods": ["GET", "POST"]},
            ],
        })

    def _extract_mpl_headers(self) -> Dict[str, str]:
        """Extract MPL headers from request."""
        return {
            k: v for k, v in self.headers.items()
            if k.lower().startswith("x-mpl")
        }

    def _send_json(
        self,
        data: Dict[str, Any],
        status: int = 200,
        extra_headers: Optional[Dict[str, str]] = None,
    ):
        """Send a JSON response."""
        body = json.dumps(data, indent=2, default=str).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", len(body))

        # Echo MPL headers
        if extra_headers:
            for key, value in extra_headers.items():
                self.send_header(key, value)

        self.end_headers()
        self.wfile.write(body)

    def _send_error(self, status: int, message: str):
        """Send an error response."""
        self._send_json({
            "error": message,
            "status": status,
            "timestamp": datetime.utcnow().isoformat() + "Z",
        }, status=status)

    def log_message(self, format, *args):
        """Custom log format."""
        timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
        method_path = args[0] if args else ""
        print(f"[{timestamp}] {method_path}")


def main():
    """Run the enhanced MCP demo server."""
    port = 8080
    server = HTTPServer(("0.0.0.0", port), MCPHandler)

    print(f"""
==================================================
       Enhanced MCP Demo Server v2.0.0
==================================================

Server running on http://0.0.0.0:{port}

Endpoints:
  Health & Discovery:
    GET  /health                 - Health check
    GET  /capabilities           - Server capabilities
    GET  /.well-known/ai-alpn    - AI-ALPN discovery

  Calendar:
    GET  /api/events             - List events
    POST /api/events             - Create event
    POST /api/events/query       - Query events

  Messages:
    GET  /api/messages           - List messages
    POST /api/messages           - Send message

  Tasks:
    GET  /api/tasks              - List tasks
    POST /api/tasks              - Create task
    POST /api/tasks/:id/execute  - Execute step

  Tools:
    POST /api/tools/invoke       - Invoke tool

  Notifications:
    POST /api/notifications      - Send notification

  Utility:
    POST /api/echo               - Echo request
    POST /api/reset              - Reset all data

MPL Headers:
    X-MPL-SType                  - Semantic type
    X-MPL-Profile                - QoM profile
    X-MPL-Args-SType             - Arguments SType

Press Ctrl+C to stop
==================================================
    """)

    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\nShutting down server...")
        server.shutdown()


if __name__ == "__main__":
    main()
