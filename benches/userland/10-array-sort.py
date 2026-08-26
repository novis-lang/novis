# Sorting a list of numbers, from the same seeded input on both sides.


def run(size: int, rounds: int) -> int:
    source = []
    seed = 12345
    i = 0
    while i < size:
        seed = (seed * 1103515245 + 12345) % 2147483648
        source.append(seed % 100000)
        i = i + 1

    total = 1
    rnd = 0
    while rnd < rounds:
        source[rnd % size] = total % 100000
        ordered = sorted(source)
        total = total + ordered[0] + ordered[-1]
        rnd = rnd + 1
    return total


print(run(50000, 20))
