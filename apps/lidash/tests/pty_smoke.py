#!/usr/bin/env python3
"""Read-only Linux PTY smoke: quit, Ctrl-C, pause/help, resize, and restoration.

Usage: python3 apps/lidash/tests/pty_smoke.py target/debug/lidash
No host configuration is changed. Only the child pseudoterminal is configured.
"""
import fcntl
import os
import pty
import select
import signal
import struct
import subprocess
import sys
import termios
import time


def exercise(binary, key):
    master, slave = pty.openpty()
    original = termios.tcgetattr(slave)
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 30, 120, 0, 0))
    child = subprocess.Popen([binary, "tui", "--interval-ms", "60000"], stdin=slave, stdout=slave, stderr=slave)
    output = bytearray()
    try:
        deadline = time.monotonic() + 5
        # Ratatui can position each header word with a cursor escape.
        while b"read-only" not in output or b"metrics" not in output:
            if child.poll() is not None or time.monotonic() >= deadline:
                raise AssertionError("TUI failed to initialize: " + repr(output))
            if select.select([master], [], [], 0.1)[0]:
                output.extend(os.read(master, 65536))
        os.write(master, b" ?")
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 8, 40, 0, 0))
        if isinstance(key, bytes):
            os.write(master, key)
        else:
            child.send_signal(key)
        child.wait(timeout=3)
        assert child.returncode == 0, child.returncode
        assert termios.tcgetattr(slave) == original, "terminal modes were not restored"
        while select.select([master], [], [], 0)[0]:
            output.extend(os.read(master, 65536))
        assert b"\x1b[?1049l" in output, "alternate screen was not left"
        assert b"\x1b[?25h" in output, "cursor was not restored"
    finally:
        if child.poll() is None:
            child.kill()
            child.wait()
        os.close(master)
        os.close(slave)


if __name__ == "__main__":
    for quit_key in (b"q", b"\x03", signal.SIGINT, signal.SIGTERM):
        exercise(sys.argv[1], quit_key)
    print("PASS: PTY quit, Ctrl-C, SIGINT, SIGTERM, resize and terminal restoration")
