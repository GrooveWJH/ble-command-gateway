from __future__ import annotations

import unittest

from ble.response_publisher import ResponsePublisher
from config.defaults import MAX_BLE_PAYLOAD_BYTES
from protocol.envelope import CODE_IN_PROGRESS, CODE_OK, CommandResponse, parse_response


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

    def test_chunking_with_160b_limit_no_longer_explodes(self) -> None:
        long_text = "\n".join(
            [
                "Wi-Fi: connected (SSID=Yundrone_MOffice)",
                "IP: 192.168.10.198",
                "User: orangepi",
                "Hostname: orangepi5ultra",
                "SSH: ssh (enabled=enabled, active=active)",
                "System: Linux 6.1.43-rockchip-rk3588 aarch64",
            ]
        )
        response = CommandResponse(
            request_id="req-chunk",
            ok=True,
            code=CODE_OK,
            text=long_text,
            data={
                "status": {
                    "wifi": "connected (SSID=Yundrone_MOffice)",
                    "ip": "192.168.10.198",
                    "user": "orangepi",
                    "hostname": "orangepi5ultra",
                    "ssh": "ssh (enabled=enabled, active=active)",
                    "system": "Linux 6.1.43-rockchip-rk3588 aarch64",
                }
            },
        )
        chunks = list(ResponsePublisher._chunk_response_if_needed(response, 160))
        parsed_chunks = [parse_response(bytes(chunk)) for chunk in chunks]

        self.assertGreater(len(chunks), 1)
        self.assertLess(len(chunks), 20)
        for idx, item in enumerate(parsed_chunks):
            self.assertLessEqual(len(bytes(chunks[idx])), 160)
            self.assertIn("chunk", item.data or {})

        self.assertEqual("".join(item.text for item in parsed_chunks), long_text)

    def test_chunking_drops_data_when_data_itself_exceeds_limit(self) -> None:
        response = CommandResponse(
            request_id="req-large-data",
            ok=True,
            code=CODE_OK,
            text="status: ok",
            data={"status": {"blob": "x" * 2000}},
        )
        chunks = list(ResponsePublisher._chunk_response_if_needed(response, 160))
        parsed_chunks = [parse_response(bytes(chunk)) for chunk in chunks]

        self.assertGreaterEqual(len(chunks), 1)
        self.assertTrue(all(len(bytes(chunk)) <= 160 for chunk in chunks))
        self.assertEqual("".join(item.text for item in parsed_chunks), "status: ok")
        for item in parsed_chunks:
            self.assertNotIn("status", item.data or {})

    def test_chunking_keeps_data_on_larger_limit(self) -> None:
        long_text = "\n".join(
            [
                "Wi-Fi: connected (SSID=LabWiFi)",
                "IP: 192.168.10.198",
                "User: orangepi",
                "Hostname: orangepi5ultra",
                "SSH: ssh (enabled=enabled, active=active)",
                "System: Linux 6.1.43-rockchip-rk3588",
            ]
        )
        response = CommandResponse(
            request_id="req-360",
            ok=True,
            code=CODE_OK,
            text=long_text,
            data={
                "status": {
                    "wifi": "connected (SSID=LabWiFi)",
                    "ip": "192.168.10.198",
                    "user": "orangepi",
                    "hostname": "orangepi5ultra",
                    "ssh": "ssh (enabled=enabled, active=active)",
                    "system": "Linux 6.1.43-rockchip-rk3588",
                }
            },
        )
        chunks = list(ResponsePublisher._chunk_response_if_needed(response, MAX_BLE_PAYLOAD_BYTES))
        parsed_chunks = [parse_response(bytes(chunk)) for chunk in chunks]

        self.assertGreater(len(chunks), 1)
        self.assertLess(len(chunks), 20)
        self.assertTrue(all(len(bytes(chunk)) <= MAX_BLE_PAYLOAD_BYTES for chunk in chunks))
        self.assertEqual("".join(item.text for item in parsed_chunks), long_text)
        for item in parsed_chunks[:-1]:
            self.assertNotIn("status", item.data or {})
        self.assertIn("status", parsed_chunks[-1].data or {})


if __name__ == "__main__":
    unittest.main()
