A list item that is a broken operator chain, a broken `->` call chain or a broken ternary breaks the
innermost list that holds it: the list's opener ends its line, as
`rule:tooling/fmt-a-list-is-one-line-or-one-item-per-line` lays out any broken list. The item then
starts its own line one level in, and its lines follow from there.

```nvs
Reader $reader = Reader::open(Document::fromHtml('<style>p { break-after: page }</style>'
    . '<p>Invoice 1042</p><p>Items: 3 bikes</p><p>Total: 1,250.00 EUR</p>')->render()->bytes);

Reader $reader = Reader::open(Document::fromHtml(
    '<style>p { break-after: page }</style>'
    . '<p>Invoice 1042</p><p>Items: 3 bikes</p><p>Total: 1,250.00 EUR</p>',
)->render()->bytes);
```

The first statement formats to the second. Only the innermost list breaks: `fromHtml`'s argument list
holds the chain, so `Reader::open(` keeps its argument on its line.

Every other item that spans lines keeps the exception the list rule already makes. An anonymous
function with a multi-line body, a nested array, a nested call's arguments and a heredoc belong to the
item, and do not break the list around them. The difference is where the item's lines start: a body
or a nested list ends with its own closer, while a chain's next line starts with an operator, which
inside an unbroken list sits under nothing.

**Declined:**
- **Leaving the list unbroken, as `nvs fmt` did.** The chain's continuation lines start inside a
  call's parentheses, and its `)` closes on the last operand's line.
- **Breaking every list up to the statement.** `Reader::open(` would end its line too, and two levels
  of indentation buy nothing the innermost one does not.
