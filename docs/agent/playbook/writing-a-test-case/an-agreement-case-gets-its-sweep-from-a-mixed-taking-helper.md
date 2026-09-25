- **An agreement case gets its sweep from a `mixed`-taking helper and `Core\Json::encode`, not from
  a closure.** Each member answers a different type and `Core\Json::encode` renders every one, so a
  `public static function same(string $name, mixed $a, mixed $b, mixed $c): int` in a `final class`
  compares renderings, echoes `DISAGREE` and the name, and returns 0 or 1. The case sums `$ok = $ok
  + Sweep::same("count", …);` per member and asserts the total. [until: gone crates/nvs-stdlib/src/json.rs:Core\Json::encode]
