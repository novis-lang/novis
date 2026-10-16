`require` is an expression, and `$config = require 'config.nvs';` is its value form. The value is whatever
the target file's own file-scope `return` returned, or `1` when the file returns nothing at all, and it
cannot be known statically.

That is exactly the case `mixed` exists for, so the value needs the same explicit `as` conversion any other
boundary value of unknown-until-runtime shape needs before it can populate a typed binding. No special case
is carved into the type system for `require`, and nothing about the construct is a second unchecked
position.
