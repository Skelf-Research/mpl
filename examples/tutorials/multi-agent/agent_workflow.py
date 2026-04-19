#!/usr/bin/env python3
"""
Multi-Agent Workflow Tutorial

Demonstrates typed multi-agent communication with MPL.
"""

import json
import uuid
import requests
from datetime import datetime, timedelta
from typing import Any, Dict, List, Optional

# MPL Proxy URL
PROXY_URL = "http://localhost:9443"


class TypedAgent:
    """An agent that communicates using MPL-typed messages."""

    def __init__(self, name: str, role: str):
        self.name = name
        self.role = role
        self.agent_id = f"agent-{uuid.uuid4().hex[:8]}"

    def create_task_plan(self, goal: str, steps: List[Dict[str, Any]]) -> Dict:
        """Create a typed task plan."""
        plan = {
            "planId": f"plan-{uuid.uuid4().hex[:8]}",
            "goal": goal,
            "steps": [
                {
                    "stepId": f"step-{i+1}",
                    "description": step["description"],
                    "tool": step.get("tool"),
                    "status": "pending"
                }
                for i, step in enumerate(steps)
            ],
            "status": "created",
            "createdBy": self.agent_id
        }

        # Send through MPL proxy
        response = requests.post(
            f"{PROXY_URL}/api/tasks",
            json=plan,
            headers={
                "Content-Type": "application/json",
                "X-MPL-SType": "org.agent.TaskPlan.v1",
            }
        )

        print(f"[{self.name}] Created task plan: {goal}")
        print(f"  Schema Fidelity: {response.headers.get('X-MPL-Schema-Fidelity', 'N/A')}")

        return response.json()

    def invoke_tool(
        self,
        tool_id: str,
        args: Dict[str, Any],
        args_stype: Optional[str] = None
    ) -> Dict:
        """Invoke a tool with typed arguments."""
        invocation = {
            "invocationId": f"inv-{uuid.uuid4().hex[:8]}",
            "toolId": tool_id,
            "args": args,
            "invokedBy": self.agent_id,
            "timestamp": datetime.utcnow().isoformat() + "Z"
        }

        if args_stype:
            invocation["argsStype"] = args_stype

        headers = {
            "Content-Type": "application/json",
            "X-MPL-SType": "org.agent.ToolInvocation.v1",
        }

        # Also validate the args if stype is specified
        if args_stype:
            headers["X-MPL-Args-SType"] = args_stype

        response = requests.post(
            f"{PROXY_URL}/api/tools/invoke",
            json=invocation,
            headers=headers
        )

        print(f"[{self.name}] Invoked tool: {tool_id}")
        print(f"  Schema Fidelity: {response.headers.get('X-MPL-Schema-Fidelity', 'N/A')}")

        return response.json()

    def process_result(self, result: Dict) -> None:
        """Process a typed tool result."""
        status = result.get("status", "unknown")
        tool_id = result.get("toolId", "unknown")

        print(f"[{self.name}] Processing result from {tool_id}")
        print(f"  Status: {status}")

        if status == "success":
            print(f"  Result: {json.dumps(result.get('result', {}), indent=4)[:200]}")
        else:
            print(f"  Error: {result.get('error', 'Unknown error')}")


def simulate_tool_execution(invocation: Dict) -> Dict:
    """Simulate tool execution (in real system, this would call actual tools)."""
    tool_id = invocation.get("toolId", "")
    args = invocation.get("args", {})

    # Simulate different tool responses
    if tool_id == "calendar.query":
        return {
            "resultId": f"res-{uuid.uuid4().hex[:8]}",
            "invocationId": invocation["invocationId"],
            "toolId": tool_id,
            "status": "success",
            "result": {
                "availableSlots": [
                    {"start": "2024-01-16T10:00:00Z", "end": "2024-01-16T11:00:00Z"},
                    {"start": "2024-01-17T14:00:00Z", "end": "2024-01-17T15:00:00Z"},
                ]
            },
            "resultStype": "data.query.Query.v1"
        }
    elif tool_id == "calendar.create":
        return {
            "resultId": f"res-{uuid.uuid4().hex[:8]}",
            "invocationId": invocation["invocationId"],
            "toolId": tool_id,
            "status": "success",
            "result": {
                "eventId": f"evt-{uuid.uuid4().hex[:8]}",
                "title": args.get("title", "Meeting"),
                "start": args.get("start"),
                "end": args.get("end"),
                "created": True
            },
            "resultStype": "org.calendar.Event.v1"
        }
    elif tool_id == "notification.send":
        return {
            "resultId": f"res-{uuid.uuid4().hex[:8]}",
            "invocationId": invocation["invocationId"],
            "toolId": tool_id,
            "status": "success",
            "result": {
                "notificationId": f"notif-{uuid.uuid4().hex[:8]}",
                "sent": True,
                "recipients": args.get("recipients", [])
            }
        }
    else:
        return {
            "resultId": f"res-{uuid.uuid4().hex[:8]}",
            "invocationId": invocation["invocationId"],
            "toolId": tool_id,
            "status": "error",
            "error": f"Unknown tool: {tool_id}"
        }


def main():
    print("=" * 60)
    print("Multi-Agent Workflow Tutorial")
    print("=" * 60)

    # Create agents
    planner = TypedAgent("Planner", "planning")
    executor = TypedAgent("Executor", "execution")

    # Step 1: Planner creates a task plan
    print("\n" + "-" * 40)
    print("Phase 1: Planning")
    print("-" * 40)

    plan = planner.create_task_plan(
        goal="Schedule a team sync meeting and notify participants",
        steps=[
            {"description": "Check calendar availability for team", "tool": "calendar.query"},
            {"description": "Create meeting event", "tool": "calendar.create"},
            {"description": "Send meeting invitations", "tool": "notification.send"},
        ]
    )

    # Step 2: Executor executes each step
    print("\n" + "-" * 40)
    print("Phase 2: Execution")
    print("-" * 40)

    # Execute step 1: Query calendar
    print("\nStep 1: Check availability")
    inv1 = executor.invoke_tool(
        tool_id="calendar.query",
        args={
            "source": "team-calendars",
            "select": ["available_slots"],
            "where": [
                {"field": "date", "operator": "gte", "value": "2024-01-16"},
                {"field": "duration", "operator": "gte", "value": 60}
            ]
        },
        args_stype="data.query.Query.v1"
    )
    result1 = simulate_tool_execution(inv1)
    executor.process_result(result1)

    # Execute step 2: Create event
    print("\nStep 2: Create meeting")
    inv2 = executor.invoke_tool(
        tool_id="calendar.create",
        args={
            "title": "Weekly Team Sync",
            "start": "2024-01-16T10:00:00Z",
            "end": "2024-01-16T11:00:00Z",
            "description": "Regular team sync to discuss progress and blockers",
            "attendees": [
                {"email": "alice@example.com", "name": "Alice"},
                {"email": "bob@example.com", "name": "Bob"},
                {"email": "charlie@example.com", "name": "Charlie"},
            ]
        },
        args_stype="org.calendar.Event.v1"
    )
    result2 = simulate_tool_execution(inv2)
    executor.process_result(result2)

    # Execute step 3: Send notifications
    print("\nStep 3: Send notifications")
    inv3 = executor.invoke_tool(
        tool_id="notification.send",
        args={
            "type": "info",
            "title": "New Meeting Scheduled",
            "message": "You've been invited to Weekly Team Sync",
            "recipients": ["alice@example.com", "bob@example.com", "charlie@example.com"]
        },
        args_stype="org.notification.Alert.v1"
    )
    result3 = simulate_tool_execution(inv3)
    executor.process_result(result3)

    # Summary
    print("\n" + "-" * 40)
    print("Phase 3: Summary")
    print("-" * 40)
    print("\nWorkflow completed successfully!")
    print(f"  Plan ID: {plan.get('planId', 'N/A')}")
    print(f"  Steps executed: 3")
    print(f"  Event created: {result2.get('result', {}).get('eventId', 'N/A')}")

    print("\n" + "=" * 60)
    print("Tutorial complete!")
    print("=" * 60)


if __name__ == "__main__":
    main()
