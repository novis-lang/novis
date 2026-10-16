<?php
// Split a text and tally it - the single most-written userland loop there is.
final class Bench {
    public static function run(int $rounds): int {
        $text = "the quick brown fox jumps over the lazy dog while the dog sleeps and the fox runs";
        $words = explode(" ", $text);
        $extras = explode(" ", "alpha beta gamma delta epsilon zeta eta theta");

        $total = 1;
        $round = 0;
        while ($round < $rounds) {
            $counts = [];
            foreach ($words as $word) {
                if (array_key_exists($word, $counts)) {
                    $counts[$word] = $counts[$word] + 1;
                } else {
                    $counts[$word] = 1;
                }
            }
            $extra = $extras[$total % 8];
            $counts[$extra] = 1;
            $total = ($total + count($counts) + max($counts)) % 1000003;
            $round = $round + 1;
        }
        return $total;
    }
}

echo Bench::run(100000), "\n";
