`build.rs` records the target, the host, the profile, `rustc --version`, the Cranelift version from
`Cargo.lock` and the commit — and no date. A build date makes two builds of the same commit differ for
no gain: the commit already answers "which source is this?" exactly, and a byte-identical rebuild is
worth more than knowing when it happened.

`NVS_BUILD_COMMIT` lets a distribution packaging Novis from a tarball supply the revision when no `.git`
is present, and every fact that cannot be determined becomes `unknown` rather than failing the build.
All of it reaches `rule:packaging/nvs-info-is-the-one-call-and-nvs-i-its-php-spelling`'s report.
