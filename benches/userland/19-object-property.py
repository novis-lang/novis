# Constructing objects and moving values through their properties - the DTO, the entity, the model.


def trunc_mod(a: int, b: int) -> int:
    """PHP and Novis truncate `%` toward zero; Python floors it, and this case's total goes negative
    on the first iteration. ADR 0100 § 5 names this as the one exception to writing each case in
    its own language's idiom -- the byte-identical gate requires the arithmetic to agree."""
    r = a % b
    return r - b if r and (a < 0) != (b < 0) else r


class Point:
    def __init__(self, x: int, y: int) -> None:
        self.x = x
        self.y = y


def run(rounds: int) -> int:
    total = 0
    i = 0
    while i < rounds:
        point = Point(i, i * 2)
        point.x = point.x + point.y
        point.y = point.y - 1
        total = trunc_mod(total + point.x + point.y, 1000003)
        i = i + 1
    return total


print(run(1000000))
