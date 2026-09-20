Turns an array inside out: every value becomes a key, and every key becomes the value under it. It
replaces PHP's `array_flip`.

The values have to be whole numbers or strings, because those are the only things an array can use as
a key. Every key of the result is a string, so a value that was the number 7 becomes the key `"7"`.

Two entries that share a value can only give you one entry, because an array holds one value per key.
The shared key keeps the place of the first of those entries, and its value is the key of the last of
them. An array with no entries gives you an array with no entries.
