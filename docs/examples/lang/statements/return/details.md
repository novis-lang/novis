A method with a return type other than `void` needs a value in every `return`. In such a method,
`return;` alone does not compile.

Every way through such a method has to end with a `return` or a `throw`. If one way reaches the end of
the method body, the program does not compile.

A file that another file loads with `require` can end with `return $value;` at its top level. The
`require` expression then gives that value.
