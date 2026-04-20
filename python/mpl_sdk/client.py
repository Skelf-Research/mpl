"""
Simplified MPL Client

A minimal, user-friendly interface for MPL. Designed for the 80% use case
where you just want to call tools with type safety.

Example:
    from mpl_sdk import Client, Mode

    # Simple usage - just works
    client = Client("http://localhost:9443")
    result = await client.call("calendar.create", {"title": "Meeting"})

    # With type hints (optional)
    from mpl_sdk.types import CalendarEvent
    result = await client.call(CalendarEvent, {"title": "Meeting"})
"""

from dataclasses import dataclass
from enum import Enum
from typing import Any, Dict, Optional, Type, TypeVar, Union

import aiohttp

from mpl_sdk.errors import MplError, SchemaFidelityError


class Mode(Enum):
    """Operating mode for the client."""

    DEVELOPMENT = "development"
    """Log validation errors but don't fail requests."""

    PRODUCTION = "production"
    """Enforce validation and fail on errors."""


@dataclass
class CallResult:
    """Result from a tool call."""

    data: Any
    """The response payload."""

    stype: Optional[str] = None
    """SType of the response, if known."""

    valid: bool = True
    """Whether schema validation passed."""

    qom_passed: bool = True
    """Whether QoM evaluation passed."""


T = TypeVar("T")


class Client:
    """
    Simple MPL client for calling typed tools.

    Example:
        # Basic usage
        client = Client("http://localhost:9443")
        result = await client.call("tools/call", {
            "name": "calendar.create",
            "arguments": {"title": "Meeting", "start": "2024-01-15T10:00:00Z"}
        })
        print(result.data)

        # Context manager for automatic cleanup
        async with Client("http://localhost:9443") as client:
            result = await client.call("calendar.create", {...})
    """

    def __init__(
        self,
        endpoint: str,
        mode: Mode = Mode.DEVELOPMENT,
        timeout: float = 30.0,
    ):
        """
        Create a new MPL client.

        Args:
            endpoint: MPL proxy URL (e.g., "http://localhost:9443")
            mode: Operating mode (development or production)
            timeout: Request timeout in seconds
        """
        self.endpoint = endpoint.rstrip("/")
        self.mode = mode
        self.timeout = timeout
        self._session: Optional[aiohttp.ClientSession] = None

    async def __aenter__(self) -> "Client":
        await self._ensure_session()
        return self

    async def __aexit__(self, *args) -> None:
        await self.close()

    async def _ensure_session(self) -> aiohttp.ClientSession:
        """Ensure HTTP session exists."""
        if self._session is None or self._session.closed:
            timeout = aiohttp.ClientTimeout(total=self.timeout)
            self._session = aiohttp.ClientSession(timeout=timeout)
        return self._session

    async def call(
        self,
        tool_or_stype: Union[str, Type[T]],
        arguments: Dict[str, Any],
        *,
        stype: Optional[str] = None,
    ) -> CallResult:
        """
        Call a tool through the MPL proxy.

        Args:
            tool_or_stype: Tool name (e.g., "calendar.create") or SType class
            arguments: Tool arguments
            stype: Override SType for the request

        Returns:
            CallResult with the response data

        Raises:
            MplError: If the request fails
            SchemaFidelityError: If validation fails in production mode
        """
        session = await self._ensure_session()

        # Determine tool name and SType
        if isinstance(tool_or_stype, str):
            tool_name = tool_or_stype
            request_stype = stype
        else:
            # Type class - extract tool name from class
            tool_name = getattr(tool_or_stype, "_tool_name", tool_or_stype.__name__)
            request_stype = stype or getattr(tool_or_stype, "_stype", None)

        # Build JSON-RPC request
        request_body = {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": tool_name,
                "arguments": arguments,
            },
        }

        headers = {}
        if request_stype:
            headers["X-MPL-SType"] = request_stype

        try:
            async with session.post(
                f"{self.endpoint}/",
                json=request_body,
                headers=headers,
            ) as response:
                # Check QoM result header
                qom_result = response.headers.get("X-MPL-QoM-Result", "pass")
                qom_passed = qom_result.lower() == "pass"

                data = await response.json()

                # Check for JSON-RPC error
                if "error" in data:
                    error = data["error"]
                    if self.mode == Mode.PRODUCTION:
                        raise MplError(
                            f"Tool call failed: {error.get('message', str(error))}"
                        )
                    return CallResult(
                        data=error,
                        valid=False,
                        qom_passed=qom_passed,
                    )

                # Extract result
                result = data.get("result", data)

                return CallResult(
                    data=result,
                    stype=response.headers.get("X-MPL-SType"),
                    valid=True,
                    qom_passed=qom_passed,
                )

        except aiohttp.ClientError as e:
            raise MplError(f"Request failed: {e}") from e

    async def send(
        self,
        stype: str,
        payload: Dict[str, Any],
    ) -> CallResult:
        """
        Send a typed payload directly (without JSON-RPC wrapper).

        Use this for non-tool payloads or direct MPL communication.

        Args:
            stype: SType identifier (e.g., "org.calendar.Event.v1")
            payload: The payload data

        Returns:
            CallResult with the response
        """
        session = await self._ensure_session()

        try:
            async with session.post(
                f"{self.endpoint}/mcp",
                json=payload,
                headers={"X-MPL-SType": stype},
            ) as response:
                qom_result = response.headers.get("X-MPL-QoM-Result", "pass")
                qom_passed = qom_result.lower() == "pass"

                data = await response.json()

                return CallResult(
                    data=data,
                    stype=stype,
                    valid=response.status == 200,
                    qom_passed=qom_passed,
                )

        except aiohttp.ClientError as e:
            raise MplError(f"Request failed: {e}") from e

    async def health(self) -> Dict[str, Any]:
        """Check proxy health status."""
        session = await self._ensure_session()

        async with session.get(f"{self.endpoint}/health") as response:
            return await response.json()

    async def capabilities(self) -> Dict[str, Any]:
        """Get proxy capabilities (supported STypes, profiles, etc.)."""
        session = await self._ensure_session()

        async with session.get(f"{self.endpoint}/capabilities") as response:
            return await response.json()

    async def close(self) -> None:
        """Close the client and cleanup resources."""
        if self._session and not self._session.closed:
            await self._session.close()
            self._session = None


def typed(stype: Optional[str] = None):
    """
    Decorator to mark a function as typed with MPL.

    The decorated function will have its arguments validated
    against the specified SType schema.

    Example:
        @typed("org.calendar.Event.v1")
        async def create_event(payload: dict) -> dict:
            return {"id": "event-123", **payload}

        # Or with auto-inferred SType from type hints
        @typed
        async def create_event(event: CalendarEvent) -> CalendarEvent:
            return CalendarEvent(id="event-123", **event.dict())
    """

    def decorator(func):
        func._mpl_stype = stype
        return func

    # Support @typed without parentheses
    if callable(stype):
        func = stype
        func._mpl_stype = None
        return func

    return decorator
