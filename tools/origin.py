#!/usr/bin/env python3
"""The local origin `examples/http.nvs` talks to: one route, one answer, held long enough to be
timed.

`rule:http-server/allow-url-pins-the-address`'s outbound half is only demonstrable against something that answers, and stage 5's
acceptance check wants `status=200` and `body=ok` from a real socket rather than from a stub. This
is that something. It is a driver-side fixture, **not** part of the language: Novis's own server is
`rule:http-server/two-deployments-and-nothing-a-proxy-owns` and it is a later milestone,
so an example that needed one to exist first would be an example nobody could run today.

`tools/loop.py` starts one of these per leg for the whole acceptance sweep -- the Windows one for
the native fixtures, a second inside WSL for the Linux leg and the valgrind sweep, because a WSL
distro's loopback is its own. Run it by hand to work on the example:

    python tools/origin.py &
    target/debug/nvs.exe run examples/http.nvs

**Every answer is held for `--delay-ms` before the status line goes out, and that delay is what
makes the example's last line deterministic.** `examples/http.nvs` asks the *same* URL for an answer
inside 1ms, and the point it makes is that the budget -- not the server -- is what ends the call. An
origin on loopback that answers instantly turns that line into a race between a 1ms deadline and a
sub-millisecond round trip, which would be a fixture that fails one iteration in some. Twenty-five
milliseconds is far under the default deadline `rule:http-server/no-spelling-for-an-unbounded-wait` gives the first request and far over the
one the last request asks for, so both lines are decided by arithmetic rather than by scheduling.

The reply is written by hand rather than through `http.server`, for two reasons that are the same
reason: `crates/nvs-stdlib/src/http/transport.rs` parses a reply, so what this serves is a *test
input* to a parser, and it should be the bytes the transport's own unit tests use rather than
whatever a library version happens to emit. `BaseHTTPRequestHandler` also answers `HTTP/1.0` by
default and logs a line per request to stderr, both of which would have to be turned off anyway.

The process exits when its **stdin reaches EOF**, so it cannot outlive the driver that started it:
`loop.py` holds the pipe and closes it when the sweep is over, and a session that is killed takes
the origin with it rather than leaving a listener on 8099 for the next run to trip over.
"""

from __future__ import annotations

import argparse
import socket
import socketserver
import sys
import threading
import time

# The route, and the two bodies. `/ok` is the only path the example asks for; anything else answers
# 404 rather than 200, because an origin that says yes to everything cannot show that pinning is
# what refused a request and not the route table.
OK_PATH = "/ok"
OK_BODY = b"ok"
MISSING_BODY = b"not found"

# The request head is read to its blank line and then dropped: this origin has no route table, no
# header policy and no body handling, and the cap is here so that a client which never sends the
# blank line cannot hold a thread on an unbounded read.
HEAD_LIMIT = 8192


def reply(status, phrase, body):
    """One HTTP/1.1 reply, framed by `Content-Length` and closed by `Connection: close`.

    The framing matches what `transport.rs`'s own tests feed its parser -- a length rather than a
    chunked body -- because the transport supports both and the one worth exercising from an example
    is the one every real origin sends."""
    head = (
        f"HTTP/1.1 {status} {phrase}\r\n"
        "Content-Type: text/plain; charset=utf-8\r\n"
        f"Content-Length: {len(body)}\r\n"
        "Connection: close\r\n\r\n"
    )
    return head.encode("ascii") + body


class Handler(socketserver.StreamRequestHandler):
    """One exchange: read the head, hold, answer, close."""

    def handle(self):
        try:
            head = b""
            while b"\r\n\r\n" not in head and len(head) < HEAD_LIMIT:
                chunk = self.connection.recv(HEAD_LIMIT)
                if not chunk:
                    return
                head += chunk
            path = ""
            first = head.split(b"\r\n", 1)[0].decode("latin-1").split(" ")
            if len(first) >= 2:
                path = first[1]

            time.sleep(self.server.delay)

            if path == OK_PATH:
                self.connection.sendall(reply(200, "OK", OK_BODY))
            else:
                self.connection.sendall(reply(404, "Not Found", MISSING_BODY))
        except OSError:
            # The caller hung up -- which is the *expected* end of the example's last request, since
            # its 1ms budget expires while this handler is still in the hold above. A stack trace
            # per deadline hit would make the driver's output unreadable for a line that is passing.
            return


class Origin(socketserver.ThreadingTCPServer):
    """Threaded because the valgrind sweep runs fixtures several at a time, and because the hold
    above would otherwise serialise them into each other's deadlines."""

    allow_reuse_address = True
    daemon_threads = True

    def __init__(self, address, delay):
        super().__init__(address, Handler)
        self.delay = delay


def already_serving(host, port):
    """Whether something already answers there.

    Two cases reach this, and both are ordinary. A developer may have started an origin by hand and
    left it up, and on a WSL distro in mirrored-networking mode the loopback *is* the Windows host's,
    so the driver's native origin is already bound to the port the WSL one is about to ask for. In
    both, the port is served and there is nothing to do -- so this process idles as a lifetime
    handle rather than failing the leg over a listener that is doing its job."""
    try:
        with socket.create_connection((host, port), timeout=1):
            return True
    except OSError:
        return False


def wait_for_eof():
    """Block until stdin closes. See the module doc: this is the whole shutdown protocol."""
    try:
        while sys.stdin.buffer.read(1):
            pass
    except (OSError, ValueError):
        pass


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n", 1)[0])
    ap.add_argument("--host", default="127.0.0.1", help="the address to bind (default 127.0.0.1)")
    ap.add_argument("--port", type=int, default=8099,
                    help="the port examples/http.nvs names (default 8099)")
    ap.add_argument("--delay-ms", type=int, default=25,
                    help="how long every answer is held, in milliseconds (default 25) -- the module "
                         "doc says why this is not zero")
    args = ap.parse_args()

    if already_serving(args.host, args.port):
        print(f"origin: http://{args.host}:{args.port} already has a listener -- leaving it alone",
              flush=True)
        wait_for_eof()
        return 0

    try:
        server = Origin((args.host, args.port), args.delay_ms / 1000.0)
    except OSError as err:
        sys.stderr.write(f"origin: cannot bind {args.host}:{args.port} -- {err}\n")
        return 2

    # The readiness line, and the contract with `loop.py`: a caller that has read this line may
    # connect. Printing it before `serve_forever` is safe because the socket is listening from
    # `__init__` -- the accept loop only decides how fast a connection is picked up, not whether it
    # is refused.
    print(f"origin: serving http://{args.host}:{args.port}{OK_PATH} "
          f"({args.delay_ms}ms per answer)", flush=True)

    threading.Thread(target=serve, args=(server,), daemon=True).start()
    try:
        wait_for_eof()
    except KeyboardInterrupt:
        pass
    server.shutdown()
    server.server_close()
    return 0


def serve(server):
    try:
        server.serve_forever(poll_interval=0.05)
    except OSError:
        pass


if __name__ == "__main__":
    raise SystemExit(main())
