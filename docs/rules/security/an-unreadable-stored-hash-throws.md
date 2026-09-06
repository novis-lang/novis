A stored value outside the read roster — garbage, a refused prefix, or a hash naming any other
algorithm — **throws** rather than answering `false`, and the message never quotes the bytes
(`rule:security/secret-sinks-refuse`).

The two outcomes are different facts. "This password is wrong" is an answer about a credential; "this
column does not hold a hash I can read" is a fact about the deployment, and collapsing them into
`false` means every user of a mis-migrated table simply fails to log in, silently, until somebody
notices. A throw wakes an operator (`rule:errors/ambiguous-input-refused`).

The roster having two entries instead of one does not change that property; it moves where the
boundary sits, and this rule is the boundary's home.
