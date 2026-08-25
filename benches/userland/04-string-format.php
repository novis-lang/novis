<?php
// Templated text: sprintf-style formatting, the default way userland renders a line.
final class Bench {
    public static function run(int $rounds): int {
        $total = 0;
        $i = 0;
        while ($i < $rounds) {
            $line = sprintf("%s #%d scored %s", "user", $i, "42");
            $total = $total + strlen($line);
            $i = $i + 1;
        }
        return $total;
    }
}

echo Bench::run(300000), "\n";
