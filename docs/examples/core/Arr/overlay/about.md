Writes one array over another, so a key that is in both takes the value from the right.

You give a base array and as many layers as you like. Each layer is written over the result of the
ones before it. A key the base already has keeps the position it had and takes the new value. A key
that is new is added at the end. Every key is treated the same way, whether it is a
number or a string.

**Good to know:** a value is replaced whole. When a key holds an array on both sides, the layer's
array takes the place of the one underneath it. `Core\Arr::overlayDeep` is the method that combines
those two arrays instead.
