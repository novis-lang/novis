Adds one attribute, a name and a value, to the element that `startElement` opened last.

Call `attribute` right after `startElement`, before you write any content or child element. After
that, the start tag is closed, and `attribute` throws a `LogicError`. An element can have each
attribute name only once, so a second call with the same name also throws a `LogicError`.

The writer escapes the value. Quotes, `<`, `&` and line breaks are written as references, so a
parser that reads the document gets back exactly the text you passed. This is also why a `tainted`
value is allowed.

**The examples below** add two attributes and show how the writer escapes them, show the two
errors, and export a product list with the product data in attributes.
