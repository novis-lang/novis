Returns the text of a `Uuid`, such as `f9168c5e-ceb2-4faa-b6bf-329bf39fa1e4`. The text is always 36
characters long: 32 hexadecimal digits in groups of 8, 4, 4, 4 and 12, joined by hyphens. The letters
are always lower case, even when `Core\Uuid::parse` read them in upper case. `echo $uuid` writes the
same text.

Because one UUID always gives one text, you can compare two texts from `toString()`, or use them as
array keys. Two texts from different sources may differ only in case. After `parse` and `toString()`
they are equal. `Core\Uuid::parse` does the opposite of `toString()`.

**The examples below** print the text of a UUID, compare two UUIDs written in different case, and
count orders per customer with the text as the key.
