# Ordering records by one field - the leaderboard, the sorted table, the sorted(key=) every app has.


class Row:
    def __init__(self, name: str, score: int) -> None:
        self.name = name
        self.score = score


def run(size: int, rounds: int) -> str:
    rows = []
    seed = 12345
    i = 0
    while i < size:
        seed = (seed * 1103515245 + 12345) % 2147483648
        rows.append(Row("row-" + str(i), seed % 100000))
        i = i + 1

    total = 1
    rnd = 0
    top = ""
    while rnd < rounds:
        rows[rnd % size] = Row("edit-" + str(rnd), total % 100000)
        ordered = sorted(rows, key=lambda r: r.score)
        first = ordered[0] if ordered else None
        if first is not None:
            total = total + first.score + 1
            top = first.name
        rnd = rnd + 1
    return top + "/" + str(total)


print(run(50000, 20))
