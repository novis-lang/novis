<?php
// explode/implode round trips - how userland parses a CSV line, a path or a header.
final class Bench {
    public static function run(int $rounds): int {
        $lines = [];
        $k = 0;
        while ($k < 16) {
            $lines[] = "id" . $k . ",name,email,city,country,created,updated,status,score,notes";
            $k = $k + 1;
        }

        $total = 1;
        $i = 0;
        while ($i < $rounds) {
            $fields = explode(",", $lines[$total % 16]);
            $back = implode("|", $fields);
            $total = $total + count($fields) + strlen($back);
            $i = $i + 1;
        }
        return $total;
    }
}

echo Bench::run(200000), "\n";
