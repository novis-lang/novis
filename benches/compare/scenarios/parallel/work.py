# One part of the work: the same loop every peer runs.


def part(seed, turns):
    x = seed
    for i in range(turns):
        x = (x * 31 + i) % 1000003
    return x
