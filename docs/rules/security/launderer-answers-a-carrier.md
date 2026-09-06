A `Core` member that removes `tainted` answers its sink's **carrier type** rather than the plain type
when **both** hold: its sink launders automatically — a value reaches that sink and is transformed
with no call written at the site — and its transform is **not idempotent**, so applying it to its own
output changes the output. Otherwise it answers the plain type.

The two conditions are one question asked twice: *a second application the source does not show, of a
transform a second application changes.* Only HTML output meets both today, which is why the HTML
escape answers a carrier and every other launderer on the roster — identifier quoting, URI component
and form-value encoding, regex quoting, terminal escaping — answers a `string`, by the predicate
rather than by exemption. Their results are legitimately concatenated into a larger string, and a
carrier would force a builder API onto four classes to close a hazard whose second call is already
visible in the source.

A launderer written for a *new* sink is measured against the two conditions, not against today's
table, and a sink that acquires an automatic launder reclassifies its own member the day it does.
