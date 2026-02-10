"""Response state and BLE publish helper."""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Deque, Iterable
from collections import deque

from config.defaults import MAX_BLE_PAYLOAD_BYTES, STATUS_STANDBY
from protocol.envelope import CODE_OK, CommandResponse, encode_response
from protocol.envelope import CODE_BUSY, CODE_IN_PROGRESS, CODE_PROVISION_FAIL, CODE_PROVISION_SUCCESS


def _initial_payload() -> bytearray:
    # Use a valid protocol frame for initial read to avoid client-side JSON parse noise.
    return bytearray(
        encode_response(
            CommandResponse(
                request_id="-",
                ok=True,
                code=CODE_OK,
                text=STATUS_STANDBY,
            )
        )
    )


@dataclass
class ResponsePublisher:
    _status_text: str = STATUS_STANDBY
    _last_payload: bytearray = field(default_factory=_initial_payload)
    _payload_queue: Deque[tuple[int, int, bytearray]] = field(default_factory=deque)
    _current_chunk: tuple[int, int] | None = None

    @property
    def status_text(self) -> str:
        return self._status_text

    @property
    def last_payload(self) -> bytearray:
        return self._last_payload

    def next_payload(self, logger: Any | None = None) -> bytearray:
        current = self._last_payload
        chunk_meta = self._current_chunk or (1, 1)
        index, total = chunk_meta
        if logger and total > 1:
            logger.info("[BLE TX] deliver chunk %d/%d bytes=%d", index, total, len(current))

        # Return current chunk first, then prepare next chunk for following read.
        if self._payload_queue:
            next_index, next_total, next_payload = self._payload_queue.popleft()
            self._last_payload = next_payload
            self._current_chunk = (next_index, next_total)
        return current

    def publish(
        self,
        response: CommandResponse,
        *,
        server: Any | None,
        service_uuid: str,
        read_char_uuid: str,
        logger: Any,
    ) -> None:
        payloads = list(self._chunk_response_if_needed(response, MAX_BLE_PAYLOAD_BYTES))
        if not payloads:
            return
        self._payload_queue.clear()
        total_chunks = len(payloads)
        for idx, item in enumerate(payloads[1:], start=2):
            self._payload_queue.append((idx, total_chunks, item))
        self._last_payload = payloads[0]
        self._current_chunk = (1, total_chunks)
        if self._should_update_status(response.code):
            self._status_text = response.text
        if total_chunks > 1:
            sizes = [len(p) for p in payloads]
            logger.info(
                "[BLE TX] id=%s ok=%s code=%s chunks=%d sizes=%s text=%s payload=%s",
                response.request_id,
                response.ok,
                response.code,
                total_chunks,
                sizes,
                self._preview_text(response.text),
                self._preview_bytes(self._last_payload),
            )
        else:
            logger.info(
                "[BLE TX] id=%s ok=%s code=%s text=%s payload=%s",
                response.request_id,
                response.ok,
                response.code,
                self._preview_text(response.text),
                self._preview_bytes(self._last_payload),
            )

        if server is None:
            return

        char = server.get_characteristic(read_char_uuid)
        if char is None:
            logger.error("Read characteristic missing, unable to publish response")
            return

        char.value = self._last_payload
        try:
            server.update_value(service_uuid, read_char_uuid)
        except Exception:  # noqa: BLE001
            logger.debug("update_value failed", exc_info=True)

    @staticmethod
    def _chunk_response_if_needed(response: CommandResponse, max_bytes: int) -> Iterable[bytearray]:
        payload = encode_response(response)
        if len(payload) <= max_bytes:
            yield bytearray(payload)
            return

        base = response.text
        data = dict(response.data or {})

        def _build(text: str, index: int, total: int, *, include_final_data: bool) -> CommandResponse:
            # Keep chunk metadata on every fragment, but attach heavy response data
            # only on the final chunk when requested.
            chunk_data: dict[str, Any] = {}
            if include_final_data and index == total:
                chunk_data.update(data)
            chunk_data["chunk"] = {"index": index, "total": total}
            return CommandResponse(
                request_id=response.request_id,
                ok=response.ok,
                code=response.code,
                text=text,
                data=chunk_data,
            )

        def _final_data_fits_without_text() -> bool:
            if not data:
                return True
            probe = _build("", 1, 1, include_final_data=True)
            return len(encode_response(probe)) <= max_bytes

        if base == "":
            include_final_data = _final_data_fits_without_text()
            encoded = bytearray(
                encode_response(
                    _build(
                        "",
                        1,
                        1,
                        include_final_data=include_final_data,
                    )
                )
            )
            if len(encoded) > max_bytes:
                encoded = bytearray(encode_response(_build("", 1, 1, include_final_data=False)))
            if len(encoded) <= max_bytes:
                yield encoded
            return

        def _split_with_total_hint(total_hint: int, *, include_final_data: bool) -> list[str]:
            chunks_local: list[str] = []
            start = 0
            while start < len(base):
                low = start + 1
                high = len(base)
                best = start
                while low <= high:
                    mid = (low + high) // 2
                    candidate = base[start:mid]
                    is_final_piece = mid == len(base)
                    # Use worst-case digits for chunk index/total under this hint.
                    test = _build(
                        candidate,
                        total_hint,
                        total_hint,
                        include_final_data=include_final_data and is_final_piece,
                    )
                    if len(encode_response(test)) <= max_bytes:
                        best = mid
                        low = mid + 1
                    else:
                        high = mid - 1
                if best == start:
                    # Ensure forward progress even for very small limits.
                    best = start + 1
                chunks_local.append(base[start:best])
                start = best
            return chunks_local

        def _encode_chunks(include_final_data: bool) -> list[bytearray]:
            chunks = _split_with_total_hint(1, include_final_data=include_final_data)
            # Re-split until total count is stable with real chunk-count digits.
            for _ in range(8):
                total_hint = len(chunks)
                refined = _split_with_total_hint(total_hint, include_final_data=include_final_data)
                if len(refined) == len(chunks):
                    chunks = refined
                    break
                chunks = refined
            total = len(chunks)
            encoded_chunks: list[bytearray] = []
            for idx, text in enumerate(chunks, start=1):
                encoded = bytearray(
                    encode_response(
                        _build(
                            text,
                            idx,
                            total,
                            include_final_data=include_final_data,
                        )
                    )
                )
                encoded_chunks.append(encoded)
            return encoded_chunks

        include_final_data = _final_data_fits_without_text()
        encoded_chunks = _encode_chunks(include_final_data=include_final_data)
        if any(len(chunk) > max_bytes for chunk in encoded_chunks):
            # If data makes chunks overflow, degrade to text-only chunks.
            encoded_chunks = _encode_chunks(include_final_data=False)
        if any(len(chunk) > max_bytes for chunk in encoded_chunks):
            # Final safety net: one-char chunks without data.
            encoded_chunks = []
            total = len(base)
            for idx, char in enumerate(base, start=1):
                encoded = bytearray(encode_response(_build(char, idx, total, include_final_data=False)))
                if len(encoded) > max_bytes:
                    raise ValueError("unable to fit response chunk into max_bytes")
                encoded_chunks.append(encoded)

        for chunk in encoded_chunks:
            yield chunk

    @staticmethod
    def _should_update_status(code: str) -> bool:
        return code in {CODE_BUSY, CODE_IN_PROGRESS, CODE_PROVISION_SUCCESS, CODE_PROVISION_FAIL}

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
