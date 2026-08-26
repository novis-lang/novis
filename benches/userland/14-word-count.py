# Split a text and tally it - the single most-written userland loop there is.


def run(rounds: int) -> int:
    text = "the quick brown fox jumps over the lazy dog while the dog sleeps and the fox runs"
    words = text.split(" ")
    extras = "alpha beta gamma delta epsilon zeta eta theta".split(" ")

    total = 1
    rnd = 0
    while rnd < rounds:
        counts = {}
        for word in words:
            counts[word] = counts.get(word, 0) + 1
        counts[extras[total % 8]] = 1
        total = (total + len(counts) + max(counts.values())) % 1000003
        rnd = rnd + 1
    return total


print(run(100000))
