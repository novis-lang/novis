# Templated text: the default way userland renders a line -- an f-string where PHP reaches for sprintf.


def run(rounds: int) -> int:
    total = 0
    i = 0
    while i < rounds:
        line = f"user #{i} scored 42"
        total = total + len(line)
        i = i + 1
    return total


print(run(300000))
