---
summary: one readable rendering of any value — `dump` writes it to stderr, `render` answers it as text
keywords: var_dump, print_r, var_export, debug_zval_refcount, dump, inspect, stderr, diagnostic
---

`Core\Debug::dump` renders each argument as a typed record — `int(42)`, `string(2) "42"`, an
array as `array(n) [ … ]` with its keys, an object as `Class#id (n) { … }` with its declared
properties — and writes one per argument to the diagnostic channel: stderr in a CLI program, never
stdout, and never into a `Core\Out::capture`. `render` answers the same text as a `Core\Cli\Text`
without the trailing newline, so it can be `echo`ed. A cycle is marked `[cycle -> #1]`, a control
byte is made visible, a `secret` property renders as `[redacted]`, and the rendering is deterministic.

```nvs
<?nvs
class Point {
    public function constructor(public int $x, public int $y) {}
}
echo Core\Debug::render(42), "\n";
echo Core\Debug::render("42"), "\n";
echo Core\Debug::render(null), "\n";
array<int> $list = [10, 20];
echo Core\Debug::render($list), "\n";
array<string> $map = ["a" => "x", "b" => "y"];
echo Core\Debug::render($map), "\n";
echo Core\Debug::render(new Point(1, 2)), "\n";
Core\Debug::dump($list);
echo "the dump went to stderr\n";
```
```output
int(42)
string(2) "42"
null
array(2) [
  0 => int(10)
  1 => int(20)
]
array(2) [
  "a" => string(1) "x"
  "b" => string(1) "y"
]
Point#1 (2) {
  $x => int(1)
  $y => int(2)
}
the dump went to stderr
```
