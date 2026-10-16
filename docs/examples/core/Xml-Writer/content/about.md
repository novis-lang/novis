Writes text inside the open element, such as the name of a product or the body of a note.

The writer escapes the text for you. `&` becomes `&amp;`, `<` becomes `&lt;` and `>` becomes `&gt;`.
A parser reads these back as the original characters, so it gets back exactly the text you wrote.
This also means that a text can never add an element or an attribute to your document. It is safe to
write text that a user typed.

Text must be inside an element, so `content` throws a `LogicError` when no element is open. XML
cannot contain most control characters, such as the character with code 0. A text that contains one
also throws a `LogicError`, and nothing is written. After you write text into an element, the writer
does not indent inside that element, because added whitespace would change the text.

**The examples below** show the escaped output, the two errors, and an export of a product list.
