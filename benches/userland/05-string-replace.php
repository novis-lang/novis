<?php
// Substitution over a template - the placeholder fill every mailer and view helper does.
final class Bench {
    public static function run(int $rounds): int {
        $template = "Hello {name}, your order {order} ships to {city} on {date}.";
        $names = [];
        $k = 0;
        while ($k < 16) {
            $names[] = str_repeat("ab", $k + 1);
            $k = $k + 1;
        }

        $total = 1;
        $i = 0;
        while ($i < $rounds) {
            $line = str_replace("{name}", $names[$total % 16], $template);
            $line = str_replace("{order}", "A-1000", $line);
            $line = str_replace("{city}", "Vienna", $line);
            $line = str_replace("{date}", "2026-08-25", $line);
            $total = $total + strlen($line);
            $i = $i + 1;
        }
        return $total;
    }
}

echo Bench::run(200000), "\n";
