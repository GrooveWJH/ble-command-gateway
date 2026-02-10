from __future__ import annotations

import unittest

from client.gateway_result_adapter import GatewayResultAdapter
from client.library_models import GatewayError, GatewayErrorCode
from client.models import ResultCode


class GatewayResultAdapterTests(unittest.TestCase):
    def test_known_mapping_timeout(self) -> None:
        error = GatewayError(GatewayErrorCode.TIMEOUT, "timed out")
        result = GatewayResultAdapter.from_gateway_error(error)
        self.assertEqual(result.code, ResultCode.TIMEOUT)
        self.assertEqual(result.message, "timed out")

    def test_unknown_code_defaults_to_failed(self) -> None:
        error = GatewayError(GatewayErrorCode.BLE_ERROR, "boom")
        error.code = "something-else"  # type: ignore[assignment]
        result = GatewayResultAdapter.from_gateway_error(error)
        self.assertEqual(result.code, ResultCode.FAILED)
        self.assertEqual(result.message, "boom")


if __name__ == "__main__":
    unittest.main()
