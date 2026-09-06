A `.lspt` case is a document, a cursor, a request, and the response rendered canonically:

```
--TEST--
member completion survives an unclosed brace
--FILE--
<?nvs
class User { public string $name; public function greet(): string { return "hi"; } }
$u = new User();
$u-><|>
if (true) {
--REQUEST--
completion
--EXPECT--
greet   method    (): string
name    property  string
```

It is a sibling of `.nvst` and deliberately not an extension of it: `.nvst` runs a program and freezes
stdout, `.lspt` asks a question of a document that is usually not even valid. Sharing the *format* is
right; sharing the *suite* would make `nvs test`'s count mean two things and break the conformance
coverage guard (`rule:testing/nvst-is-separate`). The section lexer is `nvs_test`'s, extracted to a shared
module, so `--TEST--`, `--FILE--`, `--FILE <relative/path>--` and `--EXPECT--` mean exactly what they mean
in a `.nvst` case, multi-file cases included. `<|>` is the cursor, removed before analysis and reported as
an offset — exactly one per case, and none for a request that needs none.

The runner is `nvs lsp-test <paths>`, walking directories for `*.lspt` and printing `N passed, M failed`
— the line the loop's `nvs-suite` check kind already parses, so editor behaviour is gated with no change
to the driver at all.
