# `in` over a list - the membership test userland reaches for before it reaches for a set.


def run(size: int, probes: int) -> int:
    allowed = []
    i = 0
    while i < size:
        allowed.append("role-" + str(i))
        i = i + 1

    hits = 0
    probe = 0
    while probe < probes:
        if "role-" + str(probe * 7 % (size * 2)) in allowed:
            hits = hits + 1
        probe = probe + 1
    return hits


print(run(500, 20000))
