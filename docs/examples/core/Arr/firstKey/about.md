Gives you the key of the first entry of an array. It replaces PHP's `array_key_first`.

First means first in the order the entries were put in, not the smallest key. A map gives you the
name the first entry was stored under, and a list gives you `"0"`.

Every key of a Novis array is text, so the key comes back as a `string` even for a list. You can use
it to read the value, to name the entry in a message, or to remember where a walk stopped.

The array is not changed, and nothing inside it moves.

An array with no entries gives you `null`.
