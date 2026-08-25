<?php
// Building output one piece at a time — the shape of every hand-rolled template and CSV writer.
final class Bench {
    public static function run(int $rows): string {
        $out = "";
        $i = 0;
        while ($i < $rows) {
            $out .= "<tr><td>" . $i . "</td><td>row-" . $i . "</td></tr>";
            $i = $i + 1;
        }
        return $out;
    }
}

$html = Bench::run(20000);
echo strlen($html), "\n";
