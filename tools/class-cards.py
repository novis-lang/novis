"""Which `Core` classes still owe the card `rule:core-api/reference-card` gives a class.

    python tools/class-cards.py            one line per class still owing, then the count
    python tools/class-cards.py --check    the same, exit 1 while any class owes one

The one home of the answer is `CLASSES_STILL_OWING_A_CARD` in `crates/nvs-stdlib/src/registry.rs`:
the registry test `every_registry_row_carries_a_reference_card` lets exactly those classes ship
without a `ClassDoc`, and fails naming any of them the session it gains one, so the list only ever
shrinks. This script reads that list and prints it, so a session can see what is left without a
build, and goal `core-class-cards`'s acceptance check is this script printing that nothing is.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
REGISTRY = ROOT / "crates" / "nvs-stdlib" / "src" / "registry.rs"
LIST = re.compile(
    r"const CLASSES_STILL_OWING_A_CARD: &\[&str\] = &\[(?P<body>.*?)\];",
    re.S,
)
NAME = re.compile(r'r"([^"]+)"')


def owing() -> list[str]:
    """The class names the registry test still lets ship without a card."""
    match = LIST.search(REGISTRY.read_text(encoding="utf-8"))
    if match is None:
        sys.exit(f"class-cards.py: {REGISTRY.relative_to(ROOT)} has no CLASSES_STILL_OWING_A_CARD")
    return NAME.findall(match.group("body"))


def main(argv: list[str]) -> int:
    check = "--check" in argv
    names = owing()
    for name in names:
        print(f"  {name} still owes its card")
    if names:
        print(f"{len(names)} Core class(es) still owe a card")
        return 1 if check else 0
    print("every Core class carries its card")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
