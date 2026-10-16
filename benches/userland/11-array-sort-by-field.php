<?php
// Ordering records by one field - the leaderboard, the sorted table, the usort() every app has.
final class Row {
    public string $name;
    public int $score;

    public function __construct(string $name, int $score) {
        $this->name = $name;
        $this->score = $score;
    }
}

final class Bench {
    public static function run(int $size, int $rounds): string {
        $rows = [];
        $seed = 12345;
        $i = 0;
        while ($i < $size) {
            $seed = ($seed * 1103515245 + 12345) % 2147483648;
            $rows[] = new Row("row-" . $i, $seed % 100000);
            $i = $i + 1;
        }

        $total = 1;
        $round = 0;
        $top = "";
        while ($round < $rounds) {
            $rows[$round % $size] = new Row("edit-" . $round, $total % 100000);
            $sorted = $rows;
            usort($sorted, fn(Row $a, Row $b): int => $a->score <=> $b->score);
            $first = $sorted[0] ?? null;
            if ($first !== null) {
                $total = $total + $first->score + 1;
                $top = $first->name;
            }
            $round = $round + 1;
        }
        return $top . "/" . $total;
    }
}

echo Bench::run(50000, 20), "\n";
