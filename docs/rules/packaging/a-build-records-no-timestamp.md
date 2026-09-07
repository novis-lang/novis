`build.rs` records the target, the host, the profile, `rustc --version`, the Cranelift version from
`Cargo.lock`, the commit and the commit's own committer date — and no build date. A build date makes two
builds of the same commit differ for no gain: the commit already answers "which source is this?"
exactly, and a byte-identical rebuild is worth more than knowing when it happened.

The committer date is not that timestamp and does not cost that property. It is read off the commit
rather than off the clock, so every rebuild of one commit emits the same string, and it answers the one
question the hash does not: how old is this binary. It is why the banner can carry a date at all.

`NVS_BUILD_COMMIT` and `NVS_BUILD_COMMIT_DATE` let a distribution packaging Novis from a tarball supply
the revision and its date when no `.git` is present, and every fact that cannot be determined becomes
`unknown` rather than failing the build. All of it reaches `rule:packaging/nvs-info-is-the-one-call`'s
report, and the version, commit and date of it open `rule:packaging/the-banner-states-the-build`.
