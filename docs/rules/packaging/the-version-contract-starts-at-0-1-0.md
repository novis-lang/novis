Two regimes, and the tree is in the first.

**Prototyping — until 0.1.0 ships.** Update anything, at any time, for any reason. Breakage is fixed
in place; there is no classification step, no absorption ladder, no deprecation cycle, no sign-off,
and nothing to write down beyond the commit message. The contract's rules must not be read as "good
practice to start early" — that is how a prototype acquires a compatibility surface it never agreed
to (`rule:programs/no-compatibility-promise`).

**Contract — from the 0.1.0 release onward.** 0.1.0 is the release that first declares the language
complete enough for someone else to write against; which milestone carries it is the plan's to
decide. The switch is thrown once, in the commit that tags 0.1.0, and from then on
`rule:packaging/the-versioned-surface-is-enumerated`,
`rule:packaging/who-can-see-it-decides-the-release-slot` and the ladder behind them apply.

The scheme (`rule:packaging/below-1-0-the-breaking-slot-moves-left`) and the cadence
(`rule:packaging/the-sweep-is-fired-by-a-human`) apply in both regimes. The release tool refuses to
plan a version at or past 0.1.0 without an explicit flag acknowledging that the contract begins
there, and the workspace version is below it.
