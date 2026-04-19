"""
MPL Session Management

Provides session handling for MCP/A2A server connections with MPL overlay.
"""

import json
from dataclasses import dataclass, field
from typing import Any, Callable, Dict, List, Optional

import aiohttp
import websockets

from mpl_sdk._mpl_core import (
    MplEnvelope,
    QomProfile,
    SchemaValidator,
    SType,
    semantic_hash,
)
from mpl_sdk.errors import (
    ConnectionError,
    NegotiationError,
    SchemaFidelityError,
    UnknownStypeError,
)


@dataclass
class SessionConfig:
    """Configuration for an MPL session."""

    endpoint: str
    """MCP/A2A server endpoint URL."""

    stypes: List[str] = field(default_factory=list)
    """List of SType identifiers this session supports."""

    qom_profile: Optional[str] = None
    """QoM profile name to enforce (e.g., 'qom-strict-argcheck')."""

    registry_path: Optional[str] = None
    """Path to local SType registry. Defaults to ./registry."""

    timeout_ms: int = 30000
    """Request timeout in milliseconds."""

    auto_validate: bool = True
    """Automatically validate payloads against schemas."""

    auto_hash: bool = True
    """Automatically compute semantic hashes."""


@dataclass
class NegotiatedCapabilities:
    """Result of AI-ALPN handshake negotiation."""

    common_stypes: List[str]
    """STypes supported by both client and server."""

    selected_profile: Optional[str]
    """QoM profile agreed upon."""

    server_extensions: Dict[str, Any] = field(default_factory=dict)
    """Additional capabilities advertised by server."""


class Session:
    """
    MPL Session for typed communication with MCP/A2A servers.

    Example:
        async with Session(SessionConfig(
            endpoint="ws://localhost:8080/mcp",
            stypes=["org.calendar.Event.v1", "org.agent.TaskPlan.v1"],
            qom_profile="qom-basic",
        )) as session:
            # Send typed request
            response = await session.send(
                stype="org.calendar.Event.v1",
                payload={"title": "Meeting", "start": "2024-01-15T10:00:00Z", ...}
            )
    """

    def __init__(self, config: SessionConfig):
        self.config = config
        self._ws: Optional[websockets.WebSocketClientProtocol] = None
        self._http: Optional[aiohttp.ClientSession] = None
        self._validators: Dict[str, SchemaValidator] = {}
        self._negotiated: Optional[NegotiatedCapabilities] = None
        self._qom_profile: Optional[QomProfile] = None
        self._message_handlers: Dict[str, Callable] = {}
        self._connected = False

    async def __aenter__(self) -> "Session":
        await self.connect()
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb) -> None:
        await self.close()

    async def connect(self) -> NegotiatedCapabilities:
        """
        Establish connection and perform AI-ALPN handshake.

        Returns:
            NegotiatedCapabilities with common stypes and selected profile.

        Raises:
            ConnectionError: If connection fails.
            NegotiationError: If handshake fails.
        """
        endpoint = self.config.endpoint

        try:
            if endpoint.startswith("ws://") or endpoint.startswith("wss://"):
                await self._connect_websocket()
            else:
                await self._connect_http()

            # Perform AI-ALPN handshake
            self._negotiated = await self._handshake()
            self._connected = True

            # Load QoM profile if specified
            if self.config.qom_profile:
                self._qom_profile = await self._load_qom_profile(self.config.qom_profile)

            return self._negotiated

        except Exception as e:
            raise ConnectionError(
                message=f"Failed to connect to {endpoint}: {e}",
                endpoint=endpoint,
                cause=str(e),
            )

    async def _connect_websocket(self) -> None:
        """Establish WebSocket connection."""
        self._ws = await websockets.connect(
            self.config.endpoint,
            close_timeout=self.config.timeout_ms / 1000,
        )

    async def _connect_http(self) -> None:
        """Establish HTTP session."""
        timeout = aiohttp.ClientTimeout(total=self.config.timeout_ms / 1000)
        self._http = aiohttp.ClientSession(timeout=timeout)

    async def _handshake(self) -> NegotiatedCapabilities:
        """Perform AI-ALPN capability negotiation."""
        client_hello = {
            "type": "ai-alpn-hello",
            "version": "1.0",
            "stypes": self.config.stypes,
            "qom_profiles": [self.config.qom_profile] if self.config.qom_profile else [],
        }

        if self._ws:
            await self._ws.send(json.dumps(client_hello))
            response_text = await self._ws.recv()
            response = json.loads(response_text)
        elif self._http:
            async with self._http.post(
                f"{self.config.endpoint}/.well-known/ai-alpn",
                json=client_hello,
            ) as resp:
                response = await resp.json()
        else:
            raise ConnectionError(
                message="No connection established",
                endpoint=self.config.endpoint,
            )

        if response.get("type") == "ai-alpn-error":
            raise NegotiationError(
                message=response.get("message", "Handshake failed"),
                client_stypes=self.config.stypes,
                server_stypes=response.get("server_stypes", []),
                reason=response.get("reason"),
            )

        return NegotiatedCapabilities(
            common_stypes=response.get("common_stypes", []),
            selected_profile=response.get("selected_profile"),
            server_extensions=response.get("extensions", {}),
        )

    async def _load_qom_profile(self, profile_name: str) -> QomProfile:
        """Load QoM profile by name.

        Currently supports built-in profiles:
        - qom-basic: Schema Fidelity only
        - qom-strict-argcheck: SF + Instruction Compliance
        """
        # Match known profiles
        if profile_name == "qom-strict-argcheck":
            return QomProfile.strict_argcheck()
        else:
            # Default to basic profile for unknown names
            return QomProfile.basic()

    async def send(
        self,
        stype: str,
        payload: Dict[str, Any],
        validate: Optional[bool] = None,
        compute_hash: Optional[bool] = None,
    ) -> MplEnvelope:
        """
        Send a typed payload to the server.

        Args:
            stype: SType identifier (e.g., "org.calendar.Event.v1")
            payload: The payload data matching the SType schema
            validate: Override auto_validate setting
            compute_hash: Override auto_hash setting

        Returns:
            MplEnvelope containing the response

        Raises:
            SchemaFidelityError: If validation fails
            ConnectionError: If not connected
        """
        if not self._connected:
            raise ConnectionError(
                message="Session not connected",
                endpoint=self.config.endpoint,
            )

        # Convert payload to JSON string for processing
        payload_json = json.dumps(payload)

        # Validate against schema
        should_validate = validate if validate is not None else self.config.auto_validate
        if should_validate:
            await self._validate_payload(stype, payload_json)

        # Compute semantic hash
        should_hash = compute_hash if compute_hash is not None else self.config.auto_hash
        sem_hash = None
        if should_hash:
            sem_hash = semantic_hash(payload_json)

        # Create envelope
        envelope = MplEnvelope(
            stype=stype,
            payload=payload_json,
            profile=self.config.qom_profile,
        )
        if sem_hash:
            envelope.sem_hash = sem_hash

        # Send via appropriate transport
        if self._ws:
            await self._ws.send(envelope.to_json())
            response_text = await self._ws.recv()
            response = self._parse_envelope(response_text)
        elif self._http:
            async with self._http.post(
                self.config.endpoint,
                json=json.loads(envelope.to_json()),
            ) as resp:
                response_data = await resp.json()
                response = self._parse_envelope(json.dumps(response_data))
        else:
            raise ConnectionError(
                message="No connection established",
                endpoint=self.config.endpoint,
            )

        # Validate response if auto_validate is enabled
        if should_validate and response.stype:
            await self._validate_payload(response.stype, response.payload)

        return response

    def _parse_envelope(self, json_str: str) -> MplEnvelope:
        """Parse JSON string into MplEnvelope."""
        data = json.loads(json_str)
        envelope = MplEnvelope(
            stype=data.get("stype", ""),
            payload=json.dumps(data.get("payload", {})),
            args_stype=data.get("args_stype"),
            profile=data.get("profile"),
        )
        envelope.sem_hash = data.get("sem_hash")
        return envelope

    async def _validate_payload(self, stype: str, payload_json: str) -> None:
        """Validate payload against SType schema."""
        validator = await self._get_validator(stype)
        result = validator.validate(stype, payload_json)

        if not result.valid:
            raise SchemaFidelityError(
                message=f"Payload does not match schema for {stype}",
                stype=stype,
                validation_errors=[{"path": e.path, "message": e.message} for e in result.errors],
            )

    async def _get_validator(self, stype: str) -> SchemaValidator:
        """Get or create validator for SType."""
        if stype in self._validators:
            return self._validators[stype]

        # Parse SType to find schema path
        parsed = SType(stype)
        registry_path = self.config.registry_path or "./registry"
        schema_path = (
            f"{registry_path}/stypes/{parsed.namespace}/{parsed.domain}/"
            f"{parsed.name}/v{parsed.major_version}/schema.json"
        )

        try:
            with open(schema_path) as f:
                schema_json = f.read()
            validator = SchemaValidator()
            validator.register(stype, schema_json)
            self._validators[stype] = validator
            return validator
        except FileNotFoundError:
            raise UnknownStypeError(stype=stype, registry_path=registry_path)

    def on_message(self, stype: str) -> Callable:
        """
        Decorator to register a handler for incoming messages of a specific SType.

        Example:
            @session.on_message("org.agent.TaskPlan.v1")
            async def handle_task_plan(envelope: MplEnvelope):
                print(f"Received task plan: {envelope.payload}")
        """

        def decorator(func: Callable) -> Callable:
            self._message_handlers[stype] = func
            return func

        return decorator

    async def listen(self) -> None:
        """
        Start listening for incoming messages (WebSocket only).

        Dispatches messages to registered handlers based on SType.
        """
        if not self._ws:
            raise ConnectionError(
                message="listen() requires WebSocket connection",
                endpoint=self.config.endpoint,
            )

        async for message in self._ws:
            try:
                envelope = self._parse_envelope(message)
                handler = self._message_handlers.get(envelope.stype)
                if handler:
                    await handler(envelope)
            except (json.JSONDecodeError, ValueError):
                continue  # Skip malformed messages

    async def close(self) -> None:
        """Close the session and cleanup resources."""
        if self._ws:
            await self._ws.close()
            self._ws = None

        if self._http:
            await self._http.close()
            self._http = None

        self._connected = False
        self._negotiated = None

    @property
    def is_connected(self) -> bool:
        """Check if session is connected."""
        return self._connected

    @property
    def capabilities(self) -> Optional[NegotiatedCapabilities]:
        """Get negotiated capabilities, or None if not connected."""
        return self._negotiated
