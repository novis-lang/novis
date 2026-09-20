Writes one array under another, so a key that is in both keeps the value from the left.

You give a base array and as many layers as you like. Each layer is written under the result of the
ones before it. A key the base already has is left alone, with the value and the position it had. A
key that is new is added at the end, in the order the layers give it. Every key is treated the same
way, whether it is a number or a string. It replaces PHP's `array + array`, which does not compile
here.

**Good to know:** this is `Core\Arr::overlay` with the two sides swapped. Use `underlay` when the
values you already have must win, and `overlay` when the new values must win.
