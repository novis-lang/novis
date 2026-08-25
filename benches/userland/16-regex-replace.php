<?php
// preg_replace over a string - the slug, the sanitiser, the whitespace collapse.
final class Bench {
    public static function run(int $rounds): int {
        $raws = [];
        $k = 0;
        while ($k < 16) {
            $raws[] = "  The Quick,  Brown FOX -- jumped over " . str_repeat("2 ", $k + 1) . "lazy dogs!!  ";
            $k = $k + 1;
        }

        $total = 1;
        $i = 0;
        while ($i < $rounds) {
            $slug = preg_replace('/[^A-Za-z0-9]+/', "-", $raws[$total % 16]);
            $total = $total + strlen($slug);
            $i = $i + 1;
        }
        return $total;
    }
}

echo Bench::run(200000), "\n";
