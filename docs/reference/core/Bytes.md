---
summary: octet buffers — read, search, compare, build and lay out as a wire format with `pack`/`unpack`
keywords: strlen, ord, chr, substr, strpos, strcmp, str_contains, str_starts_with, str_ends_with, str_repeat, implode, pack, unpack, binary string, byte offset, magic bytes, wire format, bytes
---

`Core\Bytes` reads and builds `bytes` — a buffer of octets with no character in it. Every position,
length and count is a **byte** offset: `length` is O(1), `at` answers one octet as a `uint`, and a
negative index counts from the end — where `Core\Str` counts graphemes. A `bytes` value cannot be
written directly in the code, and there is no `.` over buffers: a buffer is written through `Core\Encoding` (`fromHex`, `encodeText`),
joined with `join`, built with `fill` and `repeat`, and laid out as a wire format with `pack` and
`unpack`, whose code table names every field's width and byte order outright and refuses a value
outside the field's range.

```nvs
<?nvs
bytes $png = Core\Encoding::fromHex("89504e470d0a1a0a");
echo Core\Bytes::length($png), " ", Core\Bytes::at($png, 0), " ", Core\Bytes::at($png, -1), "\n";
echo Core\Bytes::startsWith($png, Core\Encoding::fromHex("89504e47")) ? "png" : "other", "\n";
echo Core\Encoding::toHex(Core\Bytes::slice($png, 1, 3)), " ", Core\Bytes::indexOf($png, Core\Encoding::fromHex("0d0a")) ?? "none", "\n";
bytes $header = Core\Bytes::pack("NnC", 305419896, 4660, 255);
echo Core\Encoding::toHex($header), "\n";
var $fields = Core\Bytes::unpack($header, "NnC");
echo Core\Json::encode($fields), "\n";
array<bytes> $parts = [$header, Core\Bytes::fill(2, 0)];
bytes $frame = Core\Bytes::join($parts);
echo Core\Bytes::length($frame), " ", Core\Encoding::toHex($frame), "\n";
echo Core\Bytes::compare(Core\Encoding::fromHex("01"), Core\Encoding::fromHex("02")), "\n";
```
```output
8 137 10
png
504e47 4
123456781234ff
[305419896,4660,255]
9 123456781234ff0000
-1
```
