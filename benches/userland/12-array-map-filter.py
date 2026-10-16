# map / filter / reduce over a list - the pipeline that replaced the for loop, spelled as
# comprehensions and `sum`, which is how Python writes it.


def run(size: int, rounds: int) -> int:
    source = list(range(size))

    total = 1
    rnd = 0
    while rnd < rounds:
        source[rnd % size] = total % 1000
        doubled = [n * 2 for n in source]
        kept = [n for n in doubled if n % 3 == 0]
        total = total + sum(kept)
        rnd = rnd + 1
    return total


print(run(100000, 20))
