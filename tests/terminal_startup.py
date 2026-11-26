"""Exercise the installed CLI with a Unix pseudo-terminal."""

import os
import pty
import subprocess
import sys
import tempfile
import termios
import unittest
from pathlib import Path


class TerminalStartup(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        self.config = Path(directory.name)
        master, self.slave = pty.openpty()
        self.addCleanup(os.close, master)
        self.addCleanup(os.close, self.slave)

    def start(self, stdout):
        original = termios.tcgetattr(self.slave)
        result = subprocess.run(
            [binary],
            stdin=self.slave,
            stdout=stdout,
            stderr=subprocess.PIPE,
            env={**os.environ, "NZI_CONFIG_DIR": str(self.config)},
            timeout=10,
        )
        self.assertNotEqual(result.returncode, 0)
        restored = termios.tcgetattr(self.slave)
        # macOS may mark pending input for reprocessing after restoring ICANON
        pending_input = getattr(termios, "PENDIN", 0)
        original[3] &= ~pending_input
        restored[3] &= ~pending_input
        self.assertEqual(restored, original)
        return result

    def test_invalid_config(self):
        (self.config / "config.toml").write_text("[", encoding="utf-8")
        result = self.start(subprocess.PIPE)
        self.assertIn(b"failed to parse config file", result.stderr)
        self.assertEqual(result.stdout, b"")

    def test_terminal_output_fails_during_setup(self):
        # raw mode succeeds on stdin before the first write to stdout fails
        reader, writer = os.pipe()
        os.close(reader)
        with os.fdopen(writer, "wb") as broken_pipe:
            result = self.start(broken_pipe)
        self.assertIn(b"Broken pipe", result.stderr)
        self.assertTrue((self.config / "config.toml").is_file())


if __name__ == "__main__":
    binary = sys.argv.pop(1)
    unittest.main()
