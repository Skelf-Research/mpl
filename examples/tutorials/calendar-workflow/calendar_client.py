#!/usr/bin/env python3
"""
Calendar Workflow Tutorial

Demonstrates typed calendar event creation with MPL validation.
"""

import json
import requests
from datetime import datetime, timedelta

# MPL Proxy URL
PROXY_URL = "http://localhost:9443"

def create_event(title: str, start: datetime, end: datetime, **kwargs) -> dict:
    """Create a calendar event through the MPL proxy."""
    payload = {
        "title": title,
        "start": start.isoformat() + "Z",
        "end": end.isoformat() + "Z",
        **kwargs
    }

    response = requests.post(
        f"{PROXY_URL}/api/events",
        json=payload,
        headers={
            "Content-Type": "application/json",
            "X-MPL-SType": "org.calendar.Event.v1",
        }
    )

    # Check MPL headers in response
    schema_fidelity = response.headers.get("X-MPL-Schema-Fidelity", "N/A")
    validation_error = response.headers.get("X-MPL-Validation-Error")

    print(f"Status: {response.status_code}")
    print(f"Schema Fidelity: {schema_fidelity}")

    if validation_error:
        print(f"Validation Error: {validation_error}")

    return response.json()


def main():
    print("=" * 50)
    print("Calendar Workflow Tutorial")
    print("=" * 50)

    # Example 1: Create a valid event
    print("\n1. Creating a valid event...")
    now = datetime.utcnow()
    event = create_event(
        title="Weekly Team Standup",
        start=now + timedelta(days=1, hours=9),
        end=now + timedelta(days=1, hours=9, minutes=30),
        description="Regular team sync to discuss progress",
        location="Conference Room A",
        attendees=[
            {"email": "alice@example.com", "name": "Alice", "status": "accepted"},
            {"email": "bob@example.com", "name": "Bob", "status": "pending"},
        ]
    )
    print(f"Created event: {json.dumps(event, indent=2)}")

    # Example 2: Create an event with minimal fields
    print("\n2. Creating minimal event...")
    event = create_event(
        title="Quick Sync",
        start=now + timedelta(days=2, hours=14),
        end=now + timedelta(days=2, hours=14, minutes=15),
    )
    print(f"Created event: {json.dumps(event, indent=2)}")

    # Example 3: Try to create an invalid event (missing required field)
    print("\n3. Attempting to create invalid event (missing end time)...")
    try:
        response = requests.post(
            f"{PROXY_URL}/api/events",
            json={
                "title": "Incomplete Event",
                "start": (now + timedelta(days=3)).isoformat() + "Z",
                # Missing "end" field - should fail validation
            },
            headers={
                "Content-Type": "application/json",
                "X-MPL-SType": "org.calendar.Event.v1",
            }
        )
        print(f"Status: {response.status_code}")
        print(f"Schema Fidelity: {response.headers.get('X-MPL-Schema-Fidelity', 'N/A')}")
        if response.headers.get("X-MPL-Validation-Error"):
            print(f"Validation Error: {response.headers['X-MPL-Validation-Error']}")
    except Exception as e:
        print(f"Error: {e}")

    # Example 4: Create all-day event
    print("\n4. Creating all-day event...")
    tomorrow = (now + timedelta(days=1)).replace(hour=0, minute=0, second=0, microsecond=0)
    event = create_event(
        title="Company Holiday",
        start=tomorrow,
        end=tomorrow + timedelta(hours=23, minutes=59, seconds=59),
    )
    print(f"Created event: {json.dumps(event, indent=2)}")

    print("\n" + "=" * 50)
    print("Tutorial complete!")
    print("=" * 50)


if __name__ == "__main__":
    main()
