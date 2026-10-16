# A substitution over a string - the slug, the sanitiser, the whitespace collapse.
import re

NON_ALNUM = re.compile(r"[^A-Za-z0-9]+")


def run(rounds: int) -> int:
    raws = []
    k = 0
    while k < 16:
        raws.append("  The Quick,  Brown FOX -- jumped over " + "2 " * (k + 1) + "lazy dogs!!  ")
        k = k + 1

    total = 1
    i = 0
    while i < rounds:
        slug = NON_ALNUM.sub("-", raws[total % 16])
        total = total + len(slug)
        i = i + 1
    return total


print(run(200000))
