<?php
// Constructing objects and moving values through their properties — the DTO, the entity, the model.
final class Point {
    public int $x;
    public int $y;

    public function __construct(int $x, int $y) {
        $this->x = $x;
        $this->y = $y;
    }
}

final class Bench {
    public static function run(int $rounds): int {
        $total = 0;
        $i = 0;
        while ($i < $rounds) {
            $point = new Point($i, $i * 2);
            $point->x = $point->x + $point->y;
            $point->y = $point->y - 1;
            $total = ($total + $point->x + $point->y) % 1000003;
            $i = $i + 1;
        }
        return $total;
    }
}

echo Bench::run(1000000), "\n";
