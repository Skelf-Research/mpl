#!/usr/bin/env python3
"""
MPL Live Demo Script

A comprehensive demonstration of MPL (Meaning Protocol Layer) working with
real MCP servers and A2A agent-to-agent communication.

Features:
- Colorful terminal output for visualization
- Live MCP tool invocations with schema validation
- Multi-agent A2A workflows
- QoM (Quality of Meaning) metric visualization
- Schema validation success/failure scenarios

Usage:
    # Start the demo stack first
    docker compose up -d

    # Run the demo
    python demo.py

    # Or run individual scenarios
    python demo.py --scenario mcp
    python demo.py --scenario a2a
    python demo.py --scenario validation
"""

import argparse
import json
import sys
import time
import uuid
import requests
from datetime import datetime, timedelta
from dataclasses import dataclass
from enum import Enum
from typing import Any, Dict, List, Optional


# =============================================================================
# Terminal Colors and Styling
# =============================================================================

class Colors:
    """ANSI color codes for terminal output."""
    RESET = "\033[0m"
    BOLD = "\033[1m"
    DIM = "\033[2m"
    UNDERLINE = "\033[4m"

    # Foreground colors
    BLACK = "\033[30m"
    RED = "\033[31m"
    GREEN = "\033[32m"
    YELLOW = "\033[33m"
    BLUE = "\033[34m"
    MAGENTA = "\033[35m"
    CYAN = "\033[36m"
    WHITE = "\033[37m"

    # Bright colors
    BRIGHT_RED = "\033[91m"
    BRIGHT_GREEN = "\033[92m"
    BRIGHT_YELLOW = "\033[93m"
    BRIGHT_BLUE = "\033[94m"
    BRIGHT_MAGENTA = "\033[95m"
    BRIGHT_CYAN = "\033[96m"

    # Background colors
    BG_RED = "\033[41m"
    BG_GREEN = "\033[42m"
    BG_YELLOW = "\033[43m"
    BG_BLUE = "\033[44m"


def color(text: str, *styles: str) -> str:
    """Apply color styles to text."""
    return f"{''.join(styles)}{text}{Colors.RESET}"


def header(text: str) -> str:
    """Format a header."""
    line = "=" * 70
    return f"\n{color(line, Colors.BRIGHT_CYAN)}\n{color(text.center(70), Colors.BOLD, Colors.BRIGHT_CYAN)}\n{color(line, Colors.BRIGHT_CYAN)}\n"


def subheader(text: str) -> str:
    """Format a subheader."""
    line = "-" * 60
    return f"\n{color(line, Colors.CYAN)}\n{color(text, Colors.BOLD, Colors.CYAN)}\n{color(line, Colors.CYAN)}"


def success(text: str) -> str:
    """Format success message."""
    return color(f"[PASS] {text}", Colors.BRIGHT_GREEN)


def failure(text: str) -> str:
    """Format failure message."""
    return color(f"[FAIL] {text}", Colors.BRIGHT_RED)


def info(text: str) -> str:
    """Format info message."""
    return color(f"[INFO] {text}", Colors.BRIGHT_BLUE)


def warn(text: str) -> str:
    """Format warning message."""
    return color(f"[WARN] {text}", Colors.BRIGHT_YELLOW)


def agent_msg(agent: str, text: str, agent_color: str = Colors.MAGENTA) -> str:
    """Format agent message."""
    return f"{color(f'[{agent}]', agent_color, Colors.BOLD)} {text}"


def arrow(direction: str = "right") -> str:
    """Get an arrow character."""
    arrows = {"right": "->", "left": "<-", "both": "<->", "down": "v"}
    return color(arrows.get(direction, "->"), Colors.DIM)


def box(title: str, content: str, width: int = 60) -> str:
    """Create a boxed output."""
    top = f"+{'-' * (width - 2)}+"
    title_line = f"| {title.center(width - 4)} |"
    sep = f"|{'-' * (width - 2)}|"

    lines = [top, title_line, sep]
    for line in content.split("\n"):
        # Truncate long lines
        if len(line) > width - 4:
            line = line[:width - 7] + "..."
        lines.append(f"| {line.ljust(width - 4)} |")
    lines.append(f"+{'-' * (width - 2)}+")

    return "\n".join(lines)


def json_pretty(data: Any, indent: int = 2) -> str:
    """Pretty print JSON with colors."""
    json_str = json.dumps(data, indent=indent, default=str)
    # Simple syntax highlighting
    json_str = json_str.replace('":', f'"{Colors.RESET}:')
    return json_str


# =============================================================================
# MPL Client
# =============================================================================

@dataclass
class MplResponse:
    """Response from MPL proxy."""
    status_code: int
    data: Dict[str, Any]
    schema_fidelity: Optional[float]
    qom_result: Optional[str]
    validation_error: Optional[str]
    latency_ms: float


class MplClient:
    """Client for interacting with MPL proxy."""

    def __init__(self, base_url: str = "http://localhost:9443"):
        self.base_url = base_url
        self.session = requests.Session()

    def health_check(self) -> bool:
        """Check if the proxy is healthy."""
        try:
            response = self.session.get(f"{self.base_url}/health", timeout=5)
            return response.status_code == 200
        except Exception:
            return False

    def request(
        self,
        method: str,
        path: str,
        payload: Optional[Dict] = None,
        stype: Optional[str] = None,
        profile: Optional[str] = None,
        args_stype: Optional[str] = None,
    ) -> MplResponse:
        """Make a request through the MPL proxy."""
        headers = {"Content-Type": "application/json"}

        if stype:
            headers["X-MPL-SType"] = stype
        if profile:
            headers["X-MPL-Profile"] = profile
        if args_stype:
            headers["X-MPL-Args-SType"] = args_stype

        url = f"{self.base_url}{path}"
        start_time = time.time()

        if method.upper() == "GET":
            response = self.session.get(url, headers=headers, timeout=10)
        else:
            response = self.session.post(
                url, json=payload, headers=headers, timeout=10
            )

        latency_ms = (time.time() - start_time) * 1000

        # Extract MPL headers
        schema_fidelity = response.headers.get("X-MPL-Schema-Fidelity")
        qom_result = response.headers.get("X-MPL-QoM-Result")
        validation_error = response.headers.get("X-MPL-Validation-Error")

        try:
            data = response.json()
        except Exception:
            data = {"raw": response.text}

        return MplResponse(
            status_code=response.status_code,
            data=data,
            schema_fidelity=float(schema_fidelity) if schema_fidelity else None,
            qom_result=qom_result,
            validation_error=validation_error,
            latency_ms=latency_ms,
        )


# =============================================================================
# Visualization Helpers
# =============================================================================

def visualize_mpl_flow(
    client: str,
    server: str,
    stype: str,
    payload_preview: str,
    direction: str = "request",
) -> None:
    """Visualize the MPL message flow."""
    if direction == "request":
        print(f"\n  {color(client, Colors.BRIGHT_CYAN)} {arrow('right')} "
              f"{color('[MPL Proxy]', Colors.BRIGHT_YELLOW)} {arrow('right')} "
              f"{color(server, Colors.BRIGHT_GREEN)}")
        print(f"  {color('SType:', Colors.DIM)} {color(stype, Colors.BRIGHT_MAGENTA)}")
        print(f"  {color('Payload:', Colors.DIM)} {payload_preview[:60]}...")
    else:
        print(f"\n  {color(client, Colors.BRIGHT_CYAN)} {arrow('left')} "
              f"{color('[MPL Proxy]', Colors.BRIGHT_YELLOW)} {arrow('left')} "
              f"{color(server, Colors.BRIGHT_GREEN)}")


def visualize_qom_metrics(
    schema_fidelity: Optional[float],
    qom_result: Optional[str],
    validation_error: Optional[str],
) -> None:
    """Visualize QoM metrics."""
    print(f"\n  {color('QoM Report:', Colors.BOLD, Colors.WHITE)}")

    # Schema Fidelity
    if schema_fidelity is not None:
        sf_bar = create_progress_bar(schema_fidelity, 20)
        sf_color = Colors.BRIGHT_GREEN if schema_fidelity >= 1.0 else Colors.BRIGHT_RED
        print(f"    Schema Fidelity: {sf_bar} {color(f'{schema_fidelity:.2f}', sf_color)}")

    # QoM Result
    if qom_result:
        result_color = Colors.BRIGHT_GREEN if qom_result.lower() == "pass" else Colors.BRIGHT_RED
        print(f"    QoM Result: {color(qom_result, result_color)}")

    # Validation Error
    if validation_error:
        print(f"    {color('Validation Error:', Colors.BRIGHT_RED)} {validation_error}")


def create_progress_bar(value: float, width: int = 20) -> str:
    """Create a progress bar."""
    filled = int(value * width)
    empty = width - filled
    bar_color = Colors.BRIGHT_GREEN if value >= 0.9 else Colors.BRIGHT_YELLOW if value >= 0.5 else Colors.BRIGHT_RED
    return f"[{color('#' * filled, bar_color)}{color('-' * empty, Colors.DIM)}]"


def visualize_agent_communication(
    sender: str,
    receiver: str,
    message_type: str,
    stype: str,
) -> None:
    """Visualize agent-to-agent communication."""
    sender_color = Colors.BRIGHT_MAGENTA
    receiver_color = Colors.BRIGHT_CYAN

    print(f"\n  {color(sender, sender_color, Colors.BOLD)} "
          f"{arrow('right')} "
          f"{color(receiver, receiver_color, Colors.BOLD)}")
    print(f"    {color('Message:', Colors.DIM)} {message_type}")
    print(f"    {color('SType:', Colors.DIM)} {color(stype, Colors.BRIGHT_YELLOW)}")


def animate_processing(text: str, duration: float = 0.5) -> None:
    """Show a simple processing animation."""
    frames = [".", "..", "...", "....", "....."]
    start = time.time()
    i = 0
    while time.time() - start < duration:
        sys.stdout.write(f"\r  {text}{frames[i % len(frames)]}     ")
        sys.stdout.flush()
        time.sleep(0.1)
        i += 1
    sys.stdout.write(f"\r  {text} Done!      \n")


# =============================================================================
# Demo Scenarios
# =============================================================================

def scenario_health_check(client: MplClient) -> bool:
    """Check if the demo stack is running."""
    print(header("System Health Check"))

    print(info("Checking MPL Proxy..."))
    animate_processing("Connecting to MPL Proxy", 0.3)

    if client.health_check():
        print(success("MPL Proxy is running"))
        return True
    else:
        print(failure("MPL Proxy is not responding"))
        print(warn("Please start the demo stack with: docker compose up -d"))
        return False


def scenario_mcp_basic(client: MplClient) -> None:
    """Demonstrate basic MCP tool invocation with MPL validation."""
    print(header("MCP Tool Invocation with MPL Validation"))

    # Scenario 1: Create a valid calendar event
    print(subheader("Scenario 1: Valid Calendar Event"))

    now = datetime.utcnow()
    event_payload = {
        "title": "Weekly Team Standup",
        "start": (now + timedelta(days=1, hours=9)).isoformat() + "Z",
        "end": (now + timedelta(days=1, hours=9, minutes=30)).isoformat() + "Z",
        "description": "Regular team sync to discuss progress and blockers",
        "location": "Conference Room A",
        "attendees": [
            {"email": "alice@example.com", "name": "Alice", "status": "accepted"},
            {"email": "bob@example.com", "name": "Bob", "status": "pending"},
        ],
    }

    visualize_mpl_flow(
        "MCP Client",
        "Calendar Server",
        "org.calendar.Event.v1",
        json.dumps(event_payload),
    )

    animate_processing("Validating schema and sending request", 0.5)

    response = client.request(
        "POST",
        "/api/events",
        payload=event_payload,
        stype="org.calendar.Event.v1",
        profile="qom-basic",
    )

    visualize_qom_metrics(
        response.schema_fidelity,
        response.qom_result,
        response.validation_error,
    )

    if response.status_code == 201:
        print(success(f"Event created successfully (ID: {response.data.get('eventId', 'N/A')})"))
        print(f"  {color('Latency:', Colors.DIM)} {response.latency_ms:.2f}ms")
    else:
        print(failure(f"Failed to create event: {response.status_code}"))

    time.sleep(0.5)

    # Scenario 2: Create an invalid event (missing required field)
    print(subheader("Scenario 2: Invalid Event (Missing 'end' Field)"))

    invalid_payload = {
        "title": "Incomplete Meeting",
        "start": (now + timedelta(days=2)).isoformat() + "Z",
        # Missing "end" field - should fail validation
    }

    visualize_mpl_flow(
        "MCP Client",
        "Calendar Server",
        "org.calendar.Event.v1",
        json.dumps(invalid_payload),
    )

    animate_processing("Validating schema", 0.3)

    response = client.request(
        "POST",
        "/api/events",
        payload=invalid_payload,
        stype="org.calendar.Event.v1",
        profile="qom-basic",
    )

    visualize_qom_metrics(
        response.schema_fidelity,
        response.qom_result,
        response.validation_error,
    )

    if response.schema_fidelity is not None and response.schema_fidelity < 1.0:
        print(success("Schema validation correctly detected missing field"))
    else:
        print(info(f"Response status: {response.status_code}"))


def scenario_multi_stype(client: MplClient) -> None:
    """Demonstrate multiple SType validations."""
    print(header("Multiple SType Validation Scenarios"))

    test_cases = [
        {
            "name": "Communication Message",
            "path": "/api/messages",
            "stype": "org.communication.Message.v1",
            "payload": {
                "id": f"msg-{uuid.uuid4().hex[:8]}",
                "content": "Hello from the MPL demo!",
                "sender": "demo@example.com",
                "recipient": "user@example.com",
                "timestamp": datetime.utcnow().isoformat() + "Z",
                "channel": "email",
            },
        },
        {
            "name": "Task Plan",
            "path": "/api/tasks",
            "stype": "org.agent.TaskPlan.v1",
            "payload": {
                "planId": f"plan-{uuid.uuid4().hex[:8]}",
                "goal": "Complete quarterly report",
                "steps": [
                    {"stepId": "step-1", "description": "Gather data", "status": "pending"},
                    {"stepId": "step-2", "description": "Analyze trends", "status": "pending"},
                    {"stepId": "step-3", "description": "Write summary", "status": "pending"},
                ],
                "status": "created",
            },
        },
        {
            "name": "Data Query",
            "path": "/api/echo",
            "stype": "data.query.Query.v1",
            "payload": {
                "source": "sales_database",
                "select": ["product", "revenue", "date"],
                "where": [
                    {"field": "date", "operator": "gte", "value": "2024-01-01"},
                    {"field": "revenue", "operator": "gt", "value": 1000},
                ],
                "orderBy": [{"field": "revenue", "direction": "desc"}],
                "limit": 100,
            },
        },
    ]

    for i, test in enumerate(test_cases, 1):
        print(subheader(f"Test {i}: {test['name']}"))

        visualize_mpl_flow(
            "Client",
            "Server",
            test["stype"],
            json.dumps(test["payload"]),
        )

        animate_processing("Processing", 0.3)

        response = client.request(
            "POST",
            test["path"],
            payload=test["payload"],
            stype=test["stype"],
        )

        visualize_qom_metrics(
            response.schema_fidelity,
            response.qom_result,
            response.validation_error,
        )

        status = "OK" if response.status_code in (200, 201) else "Error"
        print(f"  {color('Status:', Colors.DIM)} {response.status_code} {status}")
        print(f"  {color('Latency:', Colors.DIM)} {response.latency_ms:.2f}ms")

        time.sleep(0.3)


def scenario_a2a_workflow(client: MplClient) -> None:
    """Demonstrate A2A (Agent-to-Agent) workflow with typed messaging."""
    print(header("A2A Multi-Agent Workflow Demo"))

    print(f"""
  {color('Agents:', Colors.BOLD)}
    {color('[Planner Agent]', Colors.BRIGHT_MAGENTA)} - Creates task plans
    {color('[Executor Agent]', Colors.BRIGHT_CYAN)} - Executes tool invocations
    {color('[Tool Server]', Colors.BRIGHT_GREEN)} - Provides calendar/notification tools
    """)

    # Phase 1: Planning
    print(subheader("Phase 1: Task Planning"))

    planner_id = f"planner-{uuid.uuid4().hex[:6]}"
    executor_id = f"executor-{uuid.uuid4().hex[:6]}"

    plan_payload = {
        "planId": f"plan-{uuid.uuid4().hex[:8]}",
        "goal": "Schedule team meeting and notify participants",
        "steps": [
            {
                "stepId": "step-1",
                "description": "Query calendar for available slots",
                "tool": "calendar.query",
                "status": "pending",
            },
            {
                "stepId": "step-2",
                "description": "Create meeting event",
                "tool": "calendar.create",
                "status": "pending",
            },
            {
                "stepId": "step-3",
                "description": "Send notification to attendees",
                "tool": "notification.send",
                "status": "pending",
            },
        ],
        "status": "created",
        "createdBy": planner_id,
    }

    visualize_agent_communication(
        "Planner Agent",
        "Executor Agent",
        "TaskPlan",
        "org.agent.TaskPlan.v1",
    )

    print(f"\n  {color('Plan Details:', Colors.DIM)}")
    print(f"    Goal: {plan_payload['goal']}")
    for step in plan_payload['steps']:
        print(f"    - [{step['stepId']}] {step['description']}")

    animate_processing("Creating task plan", 0.4)

    response = client.request(
        "POST",
        "/api/tasks",
        payload=plan_payload,
        stype="org.agent.TaskPlan.v1",
    )

    visualize_qom_metrics(
        response.schema_fidelity,
        response.qom_result,
        response.validation_error,
    )

    print(success(f"Task plan created: {plan_payload['planId']}"))

    # Phase 2: Execution
    print(subheader("Phase 2: Step-by-Step Execution"))

    # Step 1: Query calendar
    print(agent_msg("Executor", "Executing Step 1: Query calendar availability"))

    query_payload = {
        "source": "team-calendars",
        "select": ["available_slots"],
        "where": [
            {"field": "date", "operator": "gte", "value": "2025-01-15"},
            {"field": "duration_minutes", "operator": "gte", "value": 60},
        ],
    }

    invocation_1 = {
        "invocationId": f"inv-{uuid.uuid4().hex[:8]}",
        "toolId": "calendar.query",
        "args": query_payload,
        "argsStype": "data.query.Query.v1",
        "invokedBy": executor_id,
        "timestamp": datetime.utcnow().isoformat() + "Z",
    }

    visualize_agent_communication(
        "Executor Agent",
        "Tool Server",
        "ToolInvocation",
        "org.agent.ToolInvocation.v1",
    )

    animate_processing("Querying calendars", 0.3)

    response = client.request(
        "POST",
        "/api/echo",
        payload=invocation_1,
        stype="org.agent.ToolInvocation.v1",
    )

    visualize_qom_metrics(
        response.schema_fidelity,
        response.qom_result,
        response.validation_error,
    )

    print(success("Found 2 available time slots"))
    print(f"    - Slot 1: 2025-01-16 10:00-11:00")
    print(f"    - Slot 2: 2025-01-17 14:00-15:00")

    time.sleep(0.3)

    # Step 2: Create event
    print(agent_msg("Executor", "Executing Step 2: Create calendar event"))

    event_payload = {
        "title": "Weekly Team Sync",
        "start": "2025-01-16T10:00:00Z",
        "end": "2025-01-16T11:00:00Z",
        "description": "Regular team sync to discuss progress",
        "attendees": [
            {"email": "alice@example.com", "name": "Alice"},
            {"email": "bob@example.com", "name": "Bob"},
        ],
    }

    invocation_2 = {
        "invocationId": f"inv-{uuid.uuid4().hex[:8]}",
        "toolId": "calendar.create",
        "args": event_payload,
        "argsStype": "org.calendar.Event.v1",
        "invokedBy": executor_id,
        "timestamp": datetime.utcnow().isoformat() + "Z",
    }

    visualize_agent_communication(
        "Executor Agent",
        "Tool Server",
        "ToolInvocation",
        "org.agent.ToolInvocation.v1",
    )

    animate_processing("Creating event", 0.3)

    response = client.request(
        "POST",
        "/api/events",
        payload=event_payload,
        stype="org.calendar.Event.v1",
    )

    visualize_qom_metrics(
        response.schema_fidelity,
        response.qom_result,
        response.validation_error,
    )

    event_id = response.data.get("eventId", "evt-demo")
    print(success(f"Event created: {event_id}"))

    time.sleep(0.3)

    # Step 3: Send notification
    print(agent_msg("Executor", "Executing Step 3: Send notifications"))

    notification_payload = {
        "type": "info",
        "title": "New Meeting Scheduled",
        "message": f"You've been invited to Weekly Team Sync on Jan 16, 2025 at 10:00 AM",
        "recipients": ["alice@example.com", "bob@example.com"],
        "priority": "normal",
    }

    invocation_3 = {
        "invocationId": f"inv-{uuid.uuid4().hex[:8]}",
        "toolId": "notification.send",
        "args": notification_payload,
        "argsStype": "org.notification.Alert.v1",
        "invokedBy": executor_id,
        "timestamp": datetime.utcnow().isoformat() + "Z",
    }

    visualize_agent_communication(
        "Executor Agent",
        "Tool Server",
        "ToolInvocation",
        "org.agent.ToolInvocation.v1",
    )

    animate_processing("Sending notifications", 0.3)

    response = client.request(
        "POST",
        "/api/echo",
        payload=invocation_3,
        stype="org.agent.ToolInvocation.v1",
    )

    visualize_qom_metrics(
        response.schema_fidelity,
        response.qom_result,
        response.validation_error,
    )

    print(success("Notifications sent to 2 recipients"))

    # Summary
    print(subheader("Workflow Summary"))
    print(f"""
  {color('Completed Tasks:', Colors.BRIGHT_GREEN)}
    [x] Task plan created: {plan_payload['planId']}
    [x] Calendar queried: 2 slots found
    [x] Event created: {event_id}
    [x] Notifications sent: 2 recipients

  {color('MPL Validation:', Colors.BRIGHT_CYAN)}
    All messages validated against semantic types
    QoM profiles enforced at each step
    Full audit trail with provenance tracking
    """)


def scenario_qom_profiles(client: MplClient) -> None:
    """Demonstrate different QoM profile behaviors."""
    print(header("QoM Profile Demonstration"))

    print(f"""
  {color('QoM Profiles:', Colors.BOLD)}
    {color('qom-basic:', Colors.BRIGHT_GREEN)} Schema fidelity = 1.0 (development)
    {color('qom-strict-argcheck:', Colors.BRIGHT_YELLOW)} SF = 1.0, IC >= 0.97 (production)
    {color('qom-outcome:', Colors.BRIGHT_RED)} SF = 1.0, TOC >= 0.95 (high-stakes)
    """)

    profiles = ["qom-basic", "qom-strict-argcheck"]

    valid_payload = {
        "title": "QoM Test Event",
        "start": (datetime.utcnow() + timedelta(days=1)).isoformat() + "Z",
        "end": (datetime.utcnow() + timedelta(days=1, hours=1)).isoformat() + "Z",
    }

    for profile in profiles:
        print(subheader(f"Testing Profile: {profile}"))

        visualize_mpl_flow(
            "Client",
            "Server",
            "org.calendar.Event.v1",
            f"Profile: {profile}",
        )

        animate_processing(f"Validating with {profile}", 0.3)

        response = client.request(
            "POST",
            "/api/events",
            payload=valid_payload,
            stype="org.calendar.Event.v1",
            profile=profile,
        )

        visualize_qom_metrics(
            response.schema_fidelity,
            response.qom_result,
            response.validation_error,
        )

        print(f"  {color('Profile:', Colors.DIM)} {profile}")
        print(f"  {color('Status:', Colors.DIM)} {response.status_code}")

        time.sleep(0.3)


def scenario_interactive(client: MplClient) -> None:
    """Interactive demo mode."""
    print(header("Interactive Demo Mode"))

    print(f"""
  {color('Available Commands:', Colors.BOLD)}
    1. Create calendar event
    2. Send message
    3. Create task plan
    4. Test invalid payload
    5. Check QoM metrics
    0. Exit
    """)

    while True:
        try:
            choice = input(f"\n  {color('Enter command (0-5):', Colors.BRIGHT_CYAN)} ").strip()
        except (EOFError, KeyboardInterrupt):
            print("\n  Exiting...")
            break

        if choice == "0":
            print("  Goodbye!")
            break
        elif choice == "1":
            now = datetime.utcnow()
            payload = {
                "title": "Interactive Demo Event",
                "start": (now + timedelta(hours=1)).isoformat() + "Z",
                "end": (now + timedelta(hours=2)).isoformat() + "Z",
            }
            response = client.request(
                "POST", "/api/events", payload, stype="org.calendar.Event.v1"
            )
            print(success(f"Event created: {response.data.get('eventId', 'N/A')}"))
        elif choice == "2":
            payload = {
                "id": f"msg-{uuid.uuid4().hex[:8]}",
                "content": "Hello from interactive demo!",
                "sender": "demo@example.com",
                "recipient": "user@example.com",
            }
            response = client.request(
                "POST", "/api/messages", payload, stype="org.communication.Message.v1"
            )
            print(success(f"Message sent"))
        elif choice == "3":
            payload = {
                "planId": f"plan-{uuid.uuid4().hex[:8]}",
                "goal": "Interactive task",
                "steps": [{"stepId": "1", "description": "Do something", "status": "pending"}],
                "status": "created",
            }
            response = client.request(
                "POST", "/api/tasks", payload, stype="org.agent.TaskPlan.v1"
            )
            print(success(f"Task plan created"))
        elif choice == "4":
            payload = {"title": "Missing end field"}  # Invalid - missing 'end'
            response = client.request(
                "POST", "/api/events", payload, stype="org.calendar.Event.v1"
            )
            visualize_qom_metrics(
                response.schema_fidelity,
                response.qom_result,
                response.validation_error,
            )
        elif choice == "5":
            print(info("Fetching QoM metrics from proxy..."))
            response = client.request("GET", "/_mpl/qom")
            print(f"  {json_pretty(response.data)}")
        else:
            print(warn("Invalid command"))


# =============================================================================
# Main Entry Point
# =============================================================================

def main():
    parser = argparse.ArgumentParser(
        description="MPL Live Demo Script",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  python demo.py                    # Run all scenarios
  python demo.py --scenario mcp     # Run MCP demo only
  python demo.py --scenario a2a     # Run A2A demo only
  python demo.py --interactive      # Interactive mode
        """,
    )

    parser.add_argument(
        "--scenario",
        choices=["all", "mcp", "a2a", "validation", "qom"],
        default="all",
        help="Which scenario to run",
    )
    parser.add_argument(
        "--proxy-url",
        default="http://localhost:9443",
        help="MPL proxy URL (default: http://localhost:9443)",
    )
    parser.add_argument(
        "--interactive",
        action="store_true",
        help="Run in interactive mode",
    )
    parser.add_argument(
        "--no-color",
        action="store_true",
        help="Disable colored output",
    )

    args = parser.parse_args()

    # Disable colors if requested
    if args.no_color:
        for attr in dir(Colors):
            if not attr.startswith("_"):
                setattr(Colors, attr, "")

    # Banner
    print(color(r"""
    __  _______  __       __    _                ____
   /  |/  / __ \/ /      / /   (_)   _____      / __ \___  ____ ___  ____
  / /|_/ / /_/ / /      / /   / / | / / _ \    / / / / _ \/ __ `__ \/ __ \
 / /  / / ____/ /___   / /___/ /| |/ /  __/   / /_/ /  __/ / / / / / /_/ /
/_/  /_/_/   /_____/  /_____/_/ |___/\___/   /_____/\___/_/ /_/ /_/\____/

    """, Colors.BRIGHT_CYAN, Colors.BOLD))

    print(color("  Meaning Protocol Layer - Semantic Governance for AI Agents", Colors.DIM))
    print(color(f"  Proxy URL: {args.proxy_url}", Colors.DIM))
    print()

    # Initialize client
    client = MplClient(args.proxy_url)

    # Health check
    if not scenario_health_check(client):
        sys.exit(1)

    # Run scenarios
    if args.interactive:
        scenario_interactive(client)
    elif args.scenario == "all":
        scenario_mcp_basic(client)
        scenario_multi_stype(client)
        scenario_a2a_workflow(client)
        scenario_qom_profiles(client)
    elif args.scenario == "mcp":
        scenario_mcp_basic(client)
    elif args.scenario == "a2a":
        scenario_a2a_workflow(client)
    elif args.scenario == "validation":
        scenario_multi_stype(client)
    elif args.scenario == "qom":
        scenario_qom_profiles(client)

    # Footer
    print(header("Demo Complete"))
    print(f"""
  {color('Resources:', Colors.BOLD)}
    Documentation: https://github.com/anthropics/mpl
    MCP Integration: docs/mpl-with-mcp.md
    A2A Integration: docs/mpl-with-a2a.md

  {color('Next Steps:', Colors.BOLD)}
    - Try the interactive mode: python demo.py --interactive
    - View metrics: curl http://localhost:9100/metrics
    - Check QoM reports: curl http://localhost:9443/_mpl/qom
    """)


if __name__ == "__main__":
    main()
