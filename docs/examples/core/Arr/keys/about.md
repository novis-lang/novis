Gives you every key of an array, as a list. It replaces PHP's `array_keys`.

The keys come back in the order they were put in, and the list they come back in is a fresh array
you can walk, count or change without touching the array you asked about.

Every key is text. An array with number keys gives you `"0"`, `"1"` and so on, and a list gives you
its positions written the same way. This is what the keys of a Novis array always are, so nothing is
converted on the way out.

An array with no entries gives you an array with no entries.
