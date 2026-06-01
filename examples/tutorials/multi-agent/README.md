# Multi-Agent Workflow Tutorial

This tutorial demonstrates how to use MPL for typed multi-agent communication and task delegation.

## Overview

You'll learn how to:
1. Create typed task plans (`org.agent.TaskPlan.v1`)
2. Invoke tools with typed arguments (`org.agent.ToolInvocation.v1`)
3. Handle typed tool results (`org.agent.ToolResult.v1`)
4. Track agent reasoning (`ai.agent.Reasoning.v1`)

## Prerequisites

- MPL proxy running (`docker compose up -d`)
- Python 3.10+

## The Agent STypes

### TaskPlan.v1
Represents a multi-step plan for an agent to execute:
```json
{
  "goal": "Schedule a team meeting",
  "steps": [
    {"description": "Check calendar availability", "tool": "calendar.query"},
    {"description": "Create meeting event", "tool": "calendar.create"}
  ]
}
```

### ToolInvocation.v1
A typed request to invoke a tool:
```json
{
  "toolId": "calendar.create",
  "args": {"title": "Meeting", "start": "..."},
  "argsStype": "org.calendar.Event.v1"
}
```

### ToolResult.v1
The typed result from a tool:
```json
{
  "toolId": "calendar.create",
  "status": "success",
  "result": {"eventId": "evt-123"},
  "resultStype": "org.calendar.Event.v1"
}
```

## Step 1: Run the Example

```bash
cd examples/tutorials/multi-agent
pip install -r requirements.txt
python agent_workflow.py
```

## Step 2: Understanding the Flow

```
┌─────────────┐    TaskPlan.v1     ┌─────────────┐
│  Planner    │ ─────────────────> │  Executor   │
│   Agent     │                    │   Agent     │
└─────────────┘                    └──────┬──────┘
                                          │
                           ToolInvocation.v1
                                          │
                                          ▼
                                   ┌─────────────┐
                                   │    Tool     │
                                   │   Server    │
                                   └──────┬──────┘
                                          │
                              ToolResult.v1
                                          │
                                          ▼
                                   ┌─────────────┐
                                   │  Executor   │
                                   │   Agent     │
                                   └─────────────┘
```

## What You'll See

```
Creating task plan: Schedule team sync meeting
Plan created with 3 steps

Executing step 1: Check calendar availability
Tool invocation: calendar.query
Result: 2 available slots found

Executing step 2: Create meeting event
Tool invocation: calendar.create
Schema Fidelity: 1.0
Result: Event created (evt-abc123)

Task completed successfully!
```

## QoM for Multi-Agent Systems

MPL's QoM metrics are valuable for multi-agent workflows:

- **Schema Fidelity**: Ensures agents communicate with correctly structured data
- **Instruction Compliance**: Validates that agents follow expected protocols
- **Tool Outcome Correctness**: Verifies tool results match expectations

## Next Steps

- Explore the [Registry Architecture](../../../docs/registry-architecture.md)
- Read about [QoM Profiles](../../../docs/qom-evaluation-engine.md)
- Try building your own STypes with `mpl add-stype`
