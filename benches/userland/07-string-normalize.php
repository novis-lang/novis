<?php
// trim / lower / pad - the tidy-up every form field and imported row goes through.
final class Bench {
    public static function run(int $rounds): int {
        $raws = [];
        $k = 0;
        while ($k < 16) {
            $raws[] = "   Ada  LOVELACE " . str_repeat("x", $k) . "   ";
            $k = $k + 1;
        }

        $total = 1;
        $i = 0;
        while ($i < $rounds) {
            $tidy = strtolower(trim($raws[$total % 16]));
            $padded = str_pad($tidy, 32, ".");
            $total = $total + strlen($padded);
            if (str_contains($padded, "lovelace")) {
                $total = $total + 1;
            }
            $i = $i + 1;
        }
        return $total;
    }
}

echo Bench::run(300000), "\n";
