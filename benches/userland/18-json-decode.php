<?php
// json_decode then read the fields - Novis validates into a declared type where PHP hands back a map.
final class Bench {
    public static function run(int $rounds): int {
        $payloads = [];
        $k = 0;
        while ($k < 16) {
            $payloads[] = "{\"id\":" . (4711 + $k) . ",\"name\":\"order-4711\",\"status\":\"open\",\"amount\":1999}";
            $k = $k + 1;
        }

        $total = 1;
        $i = 0;
        while ($i < $rounds) {
            $order = json_decode($payloads[$total % 16], true);
            $total = ($total + $order["id"] + $order["amount"] + strlen($order["name"])) % 1000003;
            $i = $i + 1;
        }
        return $total;
    }
}

echo Bench::run(200000), "\n";
