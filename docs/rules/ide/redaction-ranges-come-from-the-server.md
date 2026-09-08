A `secret` value is concealed in the editor by default. The language server answers a list of ranges
and their kinds; the client draws them and knows nothing about what a secret is, because the
alternative is the client guessing.

**This is a rendering, and it is not what makes a `secret` safe.** The language's guarantee is that
every sink refuses one (`rule:security/secret-sinks-refuse`), checked by the compiler, with no editor
involved and nothing an editor could weaken. Concealment covers the one exposure a compiler cannot
reach — a value legible on a screen somebody else is looking at — and it covers nothing else. An
editor drawing no decoration at all is not running a less safe language; it is only less discreet,
which is why every rule in this section is the editor's and none of them is the language's.

It does **not** ride the semantic-token channel, even though the qualifier already travels there. That
channel's contract is *names a theme styles*, and its correct degradation is to fall back to the
underlying token type — which a security default whose failure mode is *the value becomes visible*
cannot inherit. Two mechanisms, two contracts, and neither can silently disable the other.

The fail direction is named, because mid-edit is exactly when the type is unknown. A range whose
expression cannot be typed, but whose **binding's declared type carries `secret`**, is redacted
anyway; the client **holds its last answer** and never clears decorations on an error, a cancellation
or a restart. An empty answer means nothing to redact; a *missing* answer means nothing at all.

**Not on disk.** There is no language server in the tree.
