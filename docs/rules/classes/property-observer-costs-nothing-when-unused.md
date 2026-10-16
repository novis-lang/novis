Whether a class implements `PropertyObserver` is known at compile time from its declaration, so a
property with no hook on a class that does not implement it compiles to a direct field load or store:
no branch, no virtual call, nothing paid by a class that never asked for either mechanism.

"Direct" is a claim about the path a run that throws nothing takes. A landing block is not a dispatch
the access asked for — every status-returning instruction owns one, so `$obj->n = $obj->n + 1` carries
an overflow raise and a release of the receiver on mutually exclusive cold edges. Those belong to
`rule:errors/propagation`'s checked return and to `rule:types/arithmetic`'s overflow throw; counting
them here prices this rule for two other mechanisms.

The measurement rather than the assertion is what guards it: three more unhooked accesses emit no
machine-code call at all, and cost a fraction of the same accesses behind a per-property hook.
