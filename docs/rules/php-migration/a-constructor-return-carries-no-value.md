`return $value;` inside a `constructor` does not compile: the object under construction is the
result, and nothing else can be. A bare `return;` remains legal as an early exit, provided every
property is definitely assigned on that path — `rule:classes/definite-property-initialization` keeps
checking it, unchanged.

PHP 8.6 deprecates the value-returning form on the same "never made sense" argument. The fix is to
drop the value.
