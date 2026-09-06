`MAJOR.MINOR.PATCH`, **one number for the whole workspace**, and the number `nvs --version` prints
*is* the language version. Before 1.0 the breaking slot moves left by one: `0.MINOR` carries breaking
changes and `0.MINOR.PATCH` is always compatible — standard practice, and the only 0.x deviation from
SemVer.

The consequence for anything that assumes plain SemVer is that below 1.0 there is no slot left for a
compatible feature to differ from a fix in, so a "minor" and a "patch" bump agree; and a floating
container tag must never follow `0.0 → 0.1` straight across a break, so there is no bare-major tag
until there is a major. The compatible line is always `MAJOR.MINOR`, and `MAJOR` alone is a tag only
where `MAJOR` is the breaking slot.

The scheme applies in both of `rule:packaging/the-version-contract-starts-at-0-1-0`'s regimes; what
a slot *promises* is the contract's, through `rule:packaging/who-can-see-it-decides-the-release-slot`.
