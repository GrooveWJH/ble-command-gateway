from __future__ import annotations

import unittest

from client.gui.reporting import LogBuffer, normalize_report_text, strip_rich_markup


class GuiReportingTests(unittest.TestCase):
    def test_strip_rich_markup(self) -> None:
        self.assertEqual(strip_rich_markup("[bold cyan]hello[/bold cyan]"), "hello")

    def test_normalize_none(self) -> None:
        self.assertEqual(normalize_report_text(None), "")

    def test_log_buffer_overwrite_with_carriage_return(self) -> None:
        buf = LogBuffer()
        buf.append("line-1")
        buf.append("\rline-1-updated")
        self.assertEqual(buf.render(), "line-1-updated")

    def test_log_buffer_multiline(self) -> None:
        buf = LogBuffer()
        buf.append("a\nb")
        self.assertEqual(buf.render(), "a\nb")

    def test_log_buffer_clear(self) -> None:
        buf = LogBuffer()
        buf.append("line-1")
        buf.append("line-2")
        buf.clear()
        self.assertEqual(buf.render(), "")


if __name__ == "__main__":
    unittest.main()
