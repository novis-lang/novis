Gives you the key of the last entry of an array. It replaces PHP's `array_key_last`.

Last means last in the order the entries were put in, not the largest key. A map gives you the name
of the entry that was put in last, and a list gives you the last position as text.

`Core\Arr::firstKey` reads the other end of the same order. Adding a new entry changes the last key
and leaves the first key alone.

Every key of a Novis array is text, so the key comes back as a `string` even for a list. You can use
it to read the value, to name the entry in a message, or to remember where a page ended.

The array is not changed, and nothing inside it moves.

An array with no entries gives you `null`.
