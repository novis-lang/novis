# Growing a list one element at a time, then walking it - the result set of every query loop.


def run(rows: int) -> int:
    items = []
    seed = 1
    i = 0
    while i < rows:
        seed = (seed * 31 + i) % 9973
        items.append(seed)
        i = i + 1

    return sum(items) + len(items)


print(run(1000000))
