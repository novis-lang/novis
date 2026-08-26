# strip / lower / pad - the tidy-up every form field and imported row goes through.


def run(rounds: int) -> int:
    raws = []
    k = 0
    while k < 16:
        raws.append("   Ada  LOVELACE " + "x" * k + "   ")
        k = k + 1

    total = 1
    i = 0
    while i < rounds:
        tidy = raws[total % 16].strip().lower()
        padded = tidy.ljust(32, ".")
        total = total + len(padded)
        if "lovelace" in padded:
            total = total + 1
        i = i + 1
    return total


print(run(300000))
