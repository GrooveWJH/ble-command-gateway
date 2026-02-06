"""Response state and BLE publish helper."""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any

from protocol.envelope import CommandResponse, encode_response


@dataclass
class ResponsePublisher:
    _status_text: str = "Standby"
    _last_payload: bytearray = field(default_factory=lambda: bytearray(b"Standby"))

    @property
    def status_text(self) -> str:
        return self._status_text

    @property
    def last_payload(self) -> bytearray:
        return self._last_payload

    def publish(
        self,
        response: CommandResponse,
        *,
        server: Any | None,
        service_uuid: str,
        read_char_uuid: str,
        logger: Any,
    ) -> None:
        payload = bytearray(encode_response(response))
        self._last_payload = payload
        self._status_text = response.text
        logger.info(
            "[BLE TX] id=%s ok=%s code=%s text=%s payload=%s",
            response.request_id,
            response.ok,
            response.code,
            self._preview_text(response.text),
            self._preview_bytes(payload),
        )

        if server is None:
            return

        char = server.get_characteristic(read_char_uuid)
        if char is None:
            logger.error("Read characteristic missing, unable to publish response")
            return

        char.value = payload
        try:
            server.update_value(service_uuid, read_char_uuid)
        except Exception:  # noqa: BLE001
            logger.debug("update_value failed", exc_info=True)

    @staticmethod
    def preview_text(text: str, limit: int = 180) -> str:
        cleaned = text.replace("\n", "\\n")
        if len(cleaned) <= limit:
            return cleaned
        return f"{cleaned[:limit]}...(len={len(cleaned)})"

    @classmethod
    def _preview_text(cls, text: str, limit: int = 180) -> str:
        return cls.preview_text(text, limit=limit)

    @classmethod
    def _preview_bytes(cls, value: Any, limit: int = 220) -> str:
        try:
            if isinstance(value, memoryview):
                raw = value.tobytes()
            elif isinstance(value, (bytes, bytearray)):
                raw = bytes(value)
            else:
                raw = bytes(value)
            text = raw.decode("utf-8", errors="replace")
        except Exception:
            text = repr(value)
        return cls._preview_text(text, limit=limit)
