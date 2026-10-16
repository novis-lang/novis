Every `Core` member states, as a column of its signature rather than as a footnote, whether it is a
**sink** — it refuses `tainted` or `secret` — a **launderer**, whose contract names the sink it launders
for, **contagious**, where a qualified argument produces a qualified result, or **neutral**.

Taint tracking and the extension qualifier declarations both rest on that classification being *total*: a
member added without one is an incomplete member, not a member with a default. The classification lands on
the narrowest thing that has one, so for a shape parameter it is on the individual field rather than on the
parameter — a settings shape whose `host` is a sink classifies that field, while the parameter as a whole
classifies nothing (`rule:core-api/shape-flattens-at-the-abi`).

What this buys is that a reviewer reads authority off the call site: whether an argument may carry
attacker-controlled data is a property of the member being called, visible in its entry, and a member with
no answer fails the build rather than being assumed neutral.
