# A match with capture groups - how userland parses a log line, a header or an id.
import re

PATTERN = re.compile(r"(\w+) /orders/(\d+) (\d{3}) (\d+)ms")


def run(rounds: int) -> int:
    lines = []
    k = 0
    while k < 16:
        lines.append("2026-08-25 12:04:31 " + "A" * (k + 1) + " /orders/4711 200 138ms")
        k = k + 1

    total = 1
    i = 0
    while i < rounds:
        found = PATTERN.search(lines[total % 16])
        if found is not None:
            total = total + len(found.group(1))
        i = i + 1
    return total


print(run(200000))
