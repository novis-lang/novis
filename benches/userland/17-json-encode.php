<?php
// json_encode on a nested payload — what every API response ends with.
final class Bench {
    public static function run(int $rounds): int {
        $tags = ["billing", "eu", "priority"];
        $total = 0;
        $i = 0;
        while ($i < $rounds) {
            $payload = [
                "id" => $i,
                "name" => "order-" . $i,
                "status" => "open",
                "amount" => 1999,
                "tags" => $tags,
            ];
            $total = $total + strlen(json_encode($payload));
            $i = $i + 1;
        }
        return $total;
    }
}

echo Bench::run(200000), "\n";
