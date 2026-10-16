An example, an attack and a bench are attributed by **where they sit**: the trees share one relative
path per feature, so nothing has to be registered anywhere.

A test cannot work that way — a case lives where its suite wants it, and one case often pins several
features — so a test is attributed by a comment naming what it covers, written in a `.nvst` case's
file block or above a Rust test function. For a `Core` member the scan additionally credits a case
that plainly calls it by its written `::` spelling, which is what lets the cases written before this
rule count without being rewritten.

That written spelling is the only inference made. Crediting a bare `->method(` call to every class a
case happens to name is unsound rather than merely loose, and no tightening fixes it without a type
checker.
