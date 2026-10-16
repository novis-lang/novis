Reads a text of HTML and gives you its tree of elements, the same tree a web browser builds.

`Core\Html::parse` follows the parsing rules that every browser follows. It never throws an error.
HTML that is broken or incomplete is also parsed, and the rules say what the result is: a missing
`</p>` or `</li>` tag is added, and a missing `html`, `head` or `body` element is added too.

The result is a `Core\Xml\Node` for the whole document. You read it with the same methods as a tree
from `Core\Xml::parse`: `kind()`, `name()`, `attributes()`, `children()`, `text()` and `source()`.

**Good to know:** the text in the tree is `tainted`, because it comes from outside your program.
Escape it before you write it into a page.

The examples show the tags that parsing adds, the items of a list, and every link on a page.
