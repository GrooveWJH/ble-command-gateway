from __future__ import annotations

import unittest

from ble.response_publisher import ResponsePublisher
from protocol.envelope import CODE_IN_PROGRESS, CODE_OK, CommandResponse


class _DummyLogger:
    def info(self, *_args: object, **_kwargs: object) -> None:
        return

    def error(self, *_args: object, **_kwargs: object) -> None:
        return

    def debug(self, *_args: object, **_kwargs: object) -> None:
        return


class ResponsePublisherStatusTests(unittest.TestCase):
    def test_non_status_response_does_not_overwrite_status_text(self) -> None:
        publisher = ResponsePublisher()
        self.assertEqual(publisher.status_text, "Standby")

        publisher.publish(
            CommandResponse(request_id="req-1", ok=True, code=CODE_OK, text="Available commands:\n- help"),
            server=None,
            service_uuid="svc",
            read_char_uuid="char",
            logger=_DummyLogger(),
        )

        self.assertEqual(publisher.status_text, "Standby")

    def test_status_response_updates_status_text(self) -> None:
        publisher = ResponsePublisher()

        publisher.publish(
            CommandResponse(request_id="req-2", ok=True, code=CODE_IN_PROGRESS, text="Connecting"),
            server=None,
            service_uuid="svc",
            read_char_uuid="char",
            logger=_DummyLogger(),
        )

        self.assertEqual(publisher.status_text, "Connecting")


if __name__ == "__main__":
    unittest.main()
