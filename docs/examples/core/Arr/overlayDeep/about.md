Writes one array over another and combines the arrays that meet under the same key.

You give a base array and as many layers as you like, the same way `Core\Arr::overlay` takes them.
The difference is what happens when a key holds an array on both sides. `Core\Arr::overlay` replaces
the whole value. This method combines the two arrays one level further down, and then does the same
again for every key inside them, as deep as the arrays go.

A list is always replaced whole. Two lists under one key are not joined, because nothing can say
what order the result should then have. It replaces PHP's `array_replace_recursive`.

**The examples below** show two settings trees combined, a list that is replaced, and three layers
written over each other in one call.
