A class body containing `var $x;`, a bare `int $x;` or a bare `static int $x;` reports the missing
visibility and names what to write, rather than falling through to the local-declaration grammar and
reporting something about statements.

`var` is the local-inference keyword here (`rule:types/var-inference`), so PHP's `var` property form is
doubly dead; and the property grammar is looser than PHP's by accident rather than by decision — PHP
requires at least one modifier on a property, so `class A { int $x; }` is a parse error there and parses
here. Both shapes are what a porting author actually types, so both get the diagnostic that tells them the
one thing they need to change.
