# Calling an override through a base-typed handle - the interface dispatch a framework runs on.


class Shape:
    def __init__(self, size: int) -> None:
        self._size = size

    def area(self) -> int:
        return self._size * self._size


class Circle(Shape):
    def area(self) -> int:
        return self._size * self._size * 3


def run(rounds: int) -> int:
    shapes = [Shape(3), Circle(4), Shape(5), Circle(6)]
    total = 0
    i = 0
    while i < rounds:
        for shape in shapes:
            total = (total + shape.area()) % 1000003
        i = i + 1
    return total


print(run(500000))
