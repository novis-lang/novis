---
summary: a 128-bit UUID value — random v4, time-ordered v7, parsed from canonical text
keywords: uniqid, com_create_guid, uuid_is_valid, ramsey/uuid, GUID, RFC 9562, RFC 4122, v4, v7, primary key, identifier
---

A `Uuid` is an opaque 128-bit value, not a string: `v4` draws a random one, `v7` a time-ordered one
whose leading bits are a millisecond timestamp — the right database key and the wrong public
identifier, since it tells the reader when the row was made. `parse` reads only the canonical
hyphenated `8-4-4-4-12` form, in either letter case, and refuses the unhyphenated, braced and
`urn:uuid:` spellings; asking whether text is a UUID is `tryParse($s) != null`. `toString` — and
`echo` — is the only way text comes back out, always lower-case. Two `Uuid` objects compare by
identity under `==`, so two values are compared through their text.

```nvs
<?nvs
var $id = Core\Uuid::v4();
echo Core\Str::length($id->toString()), "\n";
echo Core\Str::slice($id->toString(), 14, 1), "\n";
echo Core\Str::slice(Core\Uuid::v7()->toString(), 14, 1), "\n";

var $known = Core\Uuid::parse("123E4567-E89B-12D3-A456-426614174000");
echo $known, "\n";
echo Core\Uuid::tryParse("not-a-uuid") == null ? "invalid" : "valid", "\n";
echo Core\Uuid::tryParse("123e4567-e89b-12d3-a456-426614174000") != null ? "valid" : "invalid", "\n";

var $again = Core\Uuid::parse("123e4567-e89b-12d3-a456-426614174000");
echo $known == $again ? "same object" : "different objects", "\n";
echo $known->toString() == $again->toString() ? "same value" : "different values", "\n";

try {
    Core\Uuid::parse("{123e4567-e89b-12d3-a456-426614174000}");
} catch (RuntimeError $bad) {
    echo "braces refused\n";
}
```
```output
36
4
7
123e4567-e89b-12d3-a456-426614174000
invalid
valid
different objects
same value
braces refused
```
