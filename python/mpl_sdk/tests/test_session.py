"""
Tests for MPL Session management.
"""

import json
import pytest
from unittest.mock import AsyncMock, MagicMock, patch

from mpl_sdk.session import Session, SessionConfig, NegotiatedCapabilities
from mpl_sdk.errors import ConnectionError, NegotiationError, SchemaFidelityError


class TestSessionConfig:
    """Tests for SessionConfig."""

    def test_default_config(self):
        config = SessionConfig(endpoint="ws://localhost:8080")
        assert config.endpoint == "ws://localhost:8080"
        assert config.stypes == []
        assert config.qom_profile is None
        assert config.timeout_ms == 30000
        assert config.auto_validate is True
        assert config.auto_hash is True

    def test_full_config(self):
        config = SessionConfig(
            endpoint="ws://localhost:8080",
            stypes=["org.calendar.Event.v1"],
            qom_profile="qom-basic",
            registry_path="/path/to/registry",
            timeout_ms=5000,
            auto_validate=False,
            auto_hash=False,
        )
        assert config.endpoint == "ws://localhost:8080"
        assert config.stypes == ["org.calendar.Event.v1"]
        assert config.qom_profile == "qom-basic"
        assert config.registry_path == "/path/to/registry"
        assert config.timeout_ms == 5000
        assert config.auto_validate is False
        assert config.auto_hash is False


class TestNegotiatedCapabilities:
    """Tests for NegotiatedCapabilities."""

    def test_create_capabilities(self):
        caps = NegotiatedCapabilities(
            common_stypes=["org.calendar.Event.v1"],
            selected_profile="qom-basic",
            server_extensions={"streaming": True},
        )
        assert caps.common_stypes == ["org.calendar.Event.v1"]
        assert caps.selected_profile == "qom-basic"
        assert caps.server_extensions == {"streaming": True}


class TestSession:
    """Tests for Session class."""

    @pytest.fixture
    def config(self):
        return SessionConfig(
            endpoint="ws://localhost:8080",
            stypes=["org.calendar.Event.v1"],
            qom_profile="qom-basic",
        )

    @pytest.fixture
    def http_config(self):
        return SessionConfig(
            endpoint="http://localhost:8080",
            stypes=["org.calendar.Event.v1"],
        )

    def test_session_init(self, config):
        session = Session(config)
        assert session.config == config
        assert session.is_connected is False
        assert session.capabilities is None

    @pytest.mark.asyncio
    async def test_websocket_handshake(self, config):
        """Test WebSocket AI-ALPN handshake."""
        mock_ws = AsyncMock()
        mock_ws.recv.return_value = json.dumps({
            "type": "ai-alpn-response",
            "common_stypes": ["org.calendar.Event.v1"],
            "selected_profile": "qom-basic",
            "extensions": {},
        })

        async def mock_connect(*args, **kwargs):
            return mock_ws

        with patch("websockets.connect", mock_connect):
            session = Session(config)
            caps = await session.connect()

            assert session.is_connected
            assert caps.common_stypes == ["org.calendar.Event.v1"]
            assert caps.selected_profile == "qom-basic"

            # Verify handshake message was sent
            mock_ws.send.assert_called_once()
            sent_msg = json.loads(mock_ws.send.call_args[0][0])
            assert sent_msg["type"] == "ai-alpn-hello"
            assert sent_msg["version"] == "1.0"
            assert sent_msg["stypes"] == ["org.calendar.Event.v1"]

    @pytest.mark.asyncio
    async def test_handshake_error_response(self, config):
        """Test handling of handshake error from server."""
        mock_ws = AsyncMock()
        mock_ws.recv.return_value = json.dumps({
            "type": "ai-alpn-error",
            "message": "No common stypes",
            "server_stypes": ["org.other.Type.v1"],
        })

        async def mock_connect(*args, **kwargs):
            return mock_ws

        with patch("websockets.connect", mock_connect):
            session = Session(config)
            with pytest.raises(ConnectionError):
                await session.connect()

    @pytest.mark.asyncio
    async def test_http_handshake(self, http_config):
        """Test HTTP AI-ALPN handshake."""
        mock_response = AsyncMock()
        mock_response.json.return_value = {
            "type": "ai-alpn-response",
            "common_stypes": ["org.calendar.Event.v1"],
            "selected_profile": None,
            "extensions": {},
        }

        mock_session = MagicMock()
        mock_session.post = MagicMock(return_value=AsyncMock(__aenter__=AsyncMock(return_value=mock_response)))

        with patch("aiohttp.ClientSession", return_value=mock_session):
            session = Session(http_config)
            caps = await session.connect()

            assert session.is_connected
            assert caps.common_stypes == ["org.calendar.Event.v1"]

    @pytest.mark.asyncio
    async def test_close_websocket(self, config):
        """Test closing WebSocket session."""
        mock_ws = AsyncMock()
        mock_ws.recv.return_value = json.dumps({
            "type": "ai-alpn-response",
            "common_stypes": [],
        })

        async def mock_connect(*args, **kwargs):
            return mock_ws

        with patch("websockets.connect", mock_connect):
            session = Session(config)
            await session.connect()
            await session.close()

            assert not session.is_connected
            mock_ws.close.assert_called_once()

    @pytest.mark.asyncio
    async def test_context_manager(self, config):
        """Test session as async context manager."""
        mock_ws = AsyncMock()
        mock_ws.recv.return_value = json.dumps({
            "type": "ai-alpn-response",
            "common_stypes": [],
        })

        async def mock_connect(*args, **kwargs):
            return mock_ws

        with patch("websockets.connect", mock_connect):
            async with Session(config) as session:
                assert session.is_connected

            assert not session.is_connected

    def test_parse_envelope(self, config):
        """Test envelope parsing from JSON."""
        session = Session(config)
        json_str = json.dumps({
            "stype": "org.calendar.Event.v1",
            "payload": {"title": "Meeting"},
            "sem_hash": "b3:abc123",
            "profile": "qom-basic",
        })

        envelope = session._parse_envelope(json_str)

        assert envelope.stype == "org.calendar.Event.v1"
        assert envelope.sem_hash == "b3:abc123"
        assert envelope.profile == "qom-basic"

    @pytest.mark.asyncio
    async def test_send_without_connection(self, config):
        """Test sending without connection raises error."""
        session = Session(config)
        with pytest.raises(ConnectionError):
            await session.send(
                stype="org.calendar.Event.v1",
                payload={"title": "Meeting"},
            )


class TestSessionMessageHandler:
    """Tests for session message handlers."""

    @pytest.fixture
    def config(self):
        return SessionConfig(
            endpoint="ws://localhost:8080",
            stypes=["org.calendar.Event.v1"],
        )

    def test_register_handler(self, config):
        session = Session(config)

        @session.on_message("org.calendar.Event.v1")
        async def handler(envelope):
            pass

        assert "org.calendar.Event.v1" in session._message_handlers

    @pytest.mark.asyncio
    async def test_listen_requires_websocket(self):
        """Test listen() requires WebSocket connection."""
        config = SessionConfig(endpoint="http://localhost:8080")
        session = Session(config)
        session._connected = True  # Simulate connected state

        with pytest.raises(ConnectionError):
            await session.listen()
