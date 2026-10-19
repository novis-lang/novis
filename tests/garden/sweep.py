"""A mutation sweep of HTTP Garden payloads against `nvs serve` and reference origins.

Runs inside the Garden's Docker network, in the runner image `bun nv garden` builds, beside the
Garden's own modules. Each round mutates a payload from the corpus, sends it to every origin, and
keeps the round when the references agree with each other and `nvs` reads it differently from
every one of them. Rounds that group by the same pair of readings are reported once, with a count
and the first payload that produced it.

`echo.nvs` cannot report the query string or the HTTP version, so every request is compared on
its path alone and at version 1.1.
"""

import argparse
import dataclasses
import random
import sys
import time

from diff import ErrorType, categorize_discrepancy
from fanout import fanout
from http1 import HTTPRequest, HTTPResponse
from mutations import mutate
from targets import ORIGIN_DICT

SEEDS: list[list[bytes]] = [
    [b"GET / HTTP/1.1\r\nHost: a\r\n\r\n"],
    [b"GET /a/b?c=d HTTP/1.1\r\nHost: a\r\nAccept: x\r\nAccept: y\r\n\r\n"],
    [b"POST / HTTP/1.1\r\nHost: a\r\nContent-Length: 5\r\n\r\nhello"],
    [b"POST / HTTP/1.1\r\nHost: a\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nhello\r\n0\r\n\r\n"],
    [b"GET / HTTP/1.1\r\nHost: a\r\n\r\nGET /two HTTP/1.1\r\nHost: a\r\n\r\n"],
]

CORPUS_CAP = 500


def as_nvs_sees(message: HTTPRequest | HTTPResponse) -> HTTPRequest | HTTPResponse:
    if isinstance(message, HTTPRequest):
        return dataclasses.replace(message, uri=message.uri.split(b"?", 1)[0], version=b"1.1")
    return message


def reading(stream: list[HTTPRequest | HTTPResponse]) -> str:
    if not stream:
        return "no reply"
    return ", ".join("request" if isinstance(m, HTTPRequest) else f"status {m.code.decode('latin1')}" for m in stream)


def wait_for(servers, deadline: float) -> None:
    for server in servers:
        while True:
            try:
                if any(isinstance(m, HTTPRequest) for m in server.parsed_roundtrip(SEEDS[0])):
                    break
            except OSError:
                pass
            if time.monotonic() > deadline:
                sys.exit(f"sweep: {server.name} never answered the first seed")
            time.sleep(0.5)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--subject", default="nvs")
    parser.add_argument("--refs", required=True, help="comma-separated reference origins")
    parser.add_argument("--rounds", type=int, required=True)
    parser.add_argument("--seed", type=int, required=True)
    o = parser.parse_args()

    names = [o.subject, *o.refs.split(",")]
    missing = [n for n in names if n not in ORIGIN_DICT]
    if missing:
        sys.exit(f"sweep: no running origin named {', '.join(missing)}; running: {', '.join(ORIGIN_DICT) or 'none'}")
    servers = [ORIGIN_DICT[n] for n in names]
    subject, refs = servers[0], servers[1:]
    wait_for(servers, time.monotonic() + 120)

    random.seed(o.seed)
    corpus = [list(s) for s in SEEDS]
    found: dict[tuple[str, str], tuple[int, list[bytes]]] = {}
    for done in range(1, o.rounds + 1):
        payload = random.choice(corpus)
        for _ in range(random.randint(1, 3)):
            payload = mutate(payload)
        streams = [[as_nvs_sees(m) for m in stream] for stream in fanout(payload, servers)]
        mine, theirs = streams[0], streams[1:]

        if all(categorize_discrepancy(theirs[0], t, refs[0], r) == ErrorType.OK for t, r in zip(theirs[1:], refs[1:])):
            if all(categorize_discrepancy(mine, t, subject, r) != ErrorType.OK for t, r in zip(theirs, refs)):
                key = (reading(mine), reading(theirs[0]))
                count, first = found.get(key, (0, payload))
                found[key] = (count + 1, first)
            elif len(corpus) < CORPUS_CAP:
                corpus.append(payload)

        if done % 250 == 0:
            print(f"sweep: {done} of {o.rounds} rounds, {len(found)} distinct reading(s)", file=sys.stderr, flush=True)

    print(f"{o.rounds} rounds, seed {o.seed}: {len(found)} distinct reading(s) where {o.subject} differs from {', '.join(names[1:])}")
    for (mine, theirs), (count, first) in sorted(found.items(), key=lambda kv: -kv[1][0]):
        print(f"\n{o.subject}: {mine}\nreferences: {theirs}\nrounds: {count}\nfirst payload: {first!r}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
