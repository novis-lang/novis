Every encoder that walks a value graph carries the identity of the objects on the path from the root
to the value it is writing, and the first value it meets twice ends the walk there — refused, naming
the property chain that closed the cycle, never recursing until a depth cap stops it.

The set is the **ancestor chain, not everything already seen**, because the two answer different
questions. One object held by two properties is shared rather than cyclic, and it encodes: a format
with no way to express sharing writes it out twice, which is the only answer available. Only a repeat
on the current path is a cycle. This is the one place an encoder's answer differs from a record's,
where `rule:errors/record-transformations` gives every object an identity precisely so that sharing
and cycles both survive into the rendering.

**No marker is invented in the document.** A record rendering may emit a reference because its
consumer is our own tooling; an encoder producing somebody else's payload may not, because a
Novis-specific key would make the served shape disagree with the published contract. So a cyclic
graph has no encoding, and the program is told so with the path in the message rather than with a
count of levels.

The depth cap stays and bounds what this does not: a structure that is acyclic and simply deeper than
any encoder should walk. After this rule the two failures are distinguishable, which is what was
wrong with reporting a cycle as nesting. `Core\Serialize` reaches the same property through
`rule:classes/graph-copy`'s walk rather than through this rule. A cycle costs the depth it closes at,
not the cap's worth of walking and allocation before the throw.

**Memory:** the ancestor chain is at most as deep as the walk, which the depth cap already bounds, so
the check costs nothing proportional to the size of the document, and it is released when the call
returns.

**Declined:**
- **A reference such as `{"$cycle": N}` in the encoded document.** It is right for a record rendering
  and wrong for a payload, and one encoder that sometimes emits a foreign key is worse than two whose
  audiences are stated.
- **A global seen-set.** It refuses legal documents with shared substructure and buys nothing.
- **Raising or removing the depth cap.** The cap is not what was wrong: it is what keeps the failure
  safe, and removing it trades a bad diagnosis for a stack overflow.
- **Detecting cycles in `Core\Json` alone.** Every encoder a program calls holds the rule, so one that
  is built later does not drift from the one that exists, the failure `rule:classes/graph-copy` names.
