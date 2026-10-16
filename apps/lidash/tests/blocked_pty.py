#!/usr/bin/env python3
"""Linux PTY regression for a deterministically blocked test-only sampler.

Invoked by the lidash unit-test runner with its executable path and production
sample-timeout/shutdown-grace durations. Each case gets an isolated helper process
and disposable marker. This does not simulate or validate a real kernel read.
"""

import fcntl
import os
from pathlib import Path
import pty
import select
import signal
import struct
import subprocess
import sys
import tempfile
import termios
import time


def receive(master, output, timeout):
    if select.select([master], [], [], timeout)[0]:
        output.extend(os.read(master, 65536))
        assert len(output) <= 1024 * 1024, "unexpectedly large terminal output"


def exercise(runner, label, quit_input, sample_timeout, shutdown_grace):
    with tempfile.TemporaryDirectory(prefix="lidb-blocked-pty-") as temporary:
        ready = Path(temporary) / "sampler-ready"
        environment = os.environ.copy()
        environment["LIDB_TEST_BLOCK_READY"] = str(ready)
        master, slave = pty.openpty()
        original = termios.tcgetattr(slave)
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 30, 120, 0, 0))
        child = None
        output = bytearray()
        try:
            child = subprocess.Popen(
                [
                    runner,
                    "--exact",
                    "lifecycle_tests::blocked_tui_child",
                    "--ignored",
                    "--nocapture",
                ],
                stdin=slave,
                stdout=slave,
                stderr=slave,
                env=environment,
            )
            started = time.monotonic()
            # The marker proves the callback entered; the rendered header proves raw
            # mode, signal handlers and terminal input are ready before a quit action.
            ready_deadline = started + sample_timeout * 0.8
            while not (
                ready.exists()
                and ready.read_bytes() == b"sampler entered\n"
                and b"read-only" in output
                and b"awaiting first sample" in output
            ):
                if child.poll() is not None or time.monotonic() >= ready_deadline:
                    raise AssertionError(f"{label}: blocked TUI did not initialize: {output!r}")
                receive(master, output, 0.05)
            assert ready.read_bytes() == b"sampler entered\n"
            assert b"\x1b[?1049h" in output, "alternate screen was not entered"
            assert b"\x1b[?25l" in output, "cursor was not hidden"
            raw = termios.tcgetattr(slave)
            assert not raw[3] & (termios.ICANON | termios.ECHO | termios.ISIG), "not in raw mode"

            acted = time.monotonic()
            if isinstance(quit_input, bytes):
                os.write(master, quit_input)
            elif quit_input is not None:
                child.send_signal(quit_input)

            # Quit must return well before the collection watchdog, even on slower CI.
            quit_ceiling = min(sample_timeout * 0.75, max(3.0, shutdown_grace * 20))
            deadline = (
                acted + quit_ceiling
                if quit_input is not None
                else started + sample_timeout + max(3.0, shutdown_grace * 20)
            )
            while child.poll() is None:
                if time.monotonic() >= deadline:
                    raise AssertionError(f"{label}: blocked TUI exceeded exit deadline: {output!r}")
                receive(master, output, 0.05)
            exited = time.monotonic()
            child.wait(timeout=1)
            while select.select([master], [], [], 0)[0]:
                receive(master, output, 0)

            assert child.returncode == 2, (label, child.returncode, output)
            assert termios.tcgetattr(slave) == original, "terminal modes were not exactly restored"
            diagnostic = output.index(b"lidash: collector ")
            for escape, name in (
                (b"\x1b[?1049l", "alternate screen"),
                (b"\x1b[?25h", "cursor"),
            ):
                restored = output.rfind(escape)
                assert 0 <= restored < diagnostic, f"{name} was not restored before diagnostic"
            if quit_input is None:
                assert b"collector sample timed out" in output, output
                assert exited - started >= sample_timeout - 0.25, "watchdog fired prematurely"
            else:
                assert b"collector shutdown timed out" in output, output
                assert exited - acted < quit_ceiling, "quit waited for the collection watchdog"
            print(f"PASS: {label}, exit 2, exact terminal restoration ({exited - acted:.2f}s after readiness)")
        finally:
            if child is not None and child.poll() is None:
                child.kill()
                child.wait(timeout=3)
            os.close(master)
            os.close(slave)


if __name__ == "__main__":
    runner = sys.argv[1]
    sample_timeout = float(sys.argv[2])
    shutdown_grace = float(sys.argv[3])
    for label, quit_input in (
        ("q", b"q"),
        ("raw Ctrl-C", b"\x03"),
        ("SIGINT", signal.SIGINT),
        ("SIGTERM", signal.SIGTERM),
        ("no-input collection watchdog", None),
    ):
        exercise(runner, label, quit_input, sample_timeout, shutdown_grace)
