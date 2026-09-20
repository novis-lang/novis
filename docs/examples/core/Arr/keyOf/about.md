Finds the key of the first entry that holds a value. It replaces PHP's `array_search`.

The key is returned as text, whatever the array uses. In a list the keys are the positions, so the
first entry gives `"0"`. When no entry holds the value the result is `null`. There is no `false` to
tell apart from a key of zero, which is the mistake `array_search` is known for.

Only the first entry is found. When two entries hold the same value, the one that was put in first
is the one you get.

The value is compared exactly, the same way `Core\Arr::contains` compares it. `Core\Arr::contains`
is the member to use when you only want to know whether the value is there at all.
