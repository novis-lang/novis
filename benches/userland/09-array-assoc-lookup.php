<?php
// String-keyed insert and read back — the cache, the index, the lookup table.
final class Bench {
    public static function run(int $entries): int {
        $index = [];
        $i = 0;
        while ($i < $entries) {
            $index["key-" . $i] = $i;
            $i = $i + 1;
        }

        $total = 0;
        $i = 0;
        while ($i < $entries) {
            $key = "key-" . ($i * 3 % $entries);
            if (array_key_exists($key, $index)) {
                $total = $total + $index[$key];
            }
            $i = $i + 1;
        }
        return $total;
    }
}

echo Bench::run(200000), "\n";
