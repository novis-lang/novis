Returns an array that holds one value under every key you name.

You give a list of keys and a single value. Each key becomes an entry, and every entry holds that
value. The keys come from the values of the list you pass. The keys that list uses for itself are
ignored. A key is a whole number or a text, and `1` and `"1"` are one key. Naming the same key twice
gives one entry. An empty list of keys returns an empty array. It replaces PHP's `array_fill_keys`.

When you want the keys `0`, `1`, `2` and so on instead of your own, use `Core\Arr::fill`.

**The examples below** show an empty form, a set of allowed file formats, and a tally that starts at
zero for every state an order can have.
