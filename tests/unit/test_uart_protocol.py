from __future__ import annotations

import unittest

from protocol.command_ids import CMD_PING
from protocol.envelope import (
    CODE_BAD_JSON,
    CODE_BAD_REQUEST,
    CODE_UNKNOWN_COMMAND,
    CommandParseError,
    command_request,
    encode_request,
    parse_request,
    parse_response,
    response_error,
)


class UartProtocolTests(unittest.TestCase):
    def test_roundtrip_request(self) -> None:
        req = command_request(CMD_PING, {"x": 1}, request_id="req-1")
        parsed = parse_request(encode_request(req))
        self.assertEqual(parsed.request_id, "req-1")
        self.assertEqual(parsed.command, CMD_PING)
        self.assertEqual(parsed.args["x"], 1)

    def test_invalid_json(self) -> None:
        with self.assertRaises(CommandParseError) as exc:
            parse_request("{bad json")
        self.assertEqual(exc.exception.code, CODE_BAD_JSON)

    def test_missing_cmd(self) -> None:
        with self.assertRaises(CommandParseError) as exc:
            parse_request("{\"id\": \"1\", \"args\": {}}")
        self.assertEqual(exc.exception.code, CODE_BAD_REQUEST)

    def test_parse_response_requires_fields(self) -> None:
        with self.assertRaises(CommandParseError) as exc:
            parse_response("{\"ok\": true}")
        self.assertEqual(exc.exception.code, CODE_BAD_REQUEST)

    def test_unknown_command_error_shape(self) -> None:
        error = response_error("req-x", CODE_UNKNOWN_COMMAND, "unknown")
        self.assertEqual(error.request_id, "req-x")
        self.assertFalse(error.ok)
        self.assertEqual(error.code, CODE_UNKNOWN_COMMAND)


if __name__ == "__main__":
    unittest.main()
