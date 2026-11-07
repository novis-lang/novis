Returns the HTML of the text that is not in a finished block yet, as if the document ended after the
last piece.

Show this HTML after the finished blocks. At the next `push`, replace it with the new `open` part. The
open part is usually the last block, such as a paragraph that is still being written. It can also be
a list that is still growing, or an HTML element whose end tag has not arrived. Markdown that is not
complete yet shows as text: a `**` without its closing `**` shows as two stars until the closing pair
arrives.

**Good to know:** `open` is empty until the first block starts. For example, it is empty when the text
so far has only blank lines or reference definitions.

**The examples below** show half-written bold text, a list that grows, and a chat page that shows an
answer while it arrives.
