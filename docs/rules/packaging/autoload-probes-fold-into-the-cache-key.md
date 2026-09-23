Plain `autoload` (`rule:programs/autoload`) adds no new dependency kind to the artifact cache, with
one exception. A file nobody references changes nothing, and when a reference is finally written the
*referencing* file's content hash changes and the cache key misses on its own. The exception is
**shadowing**: adding `src/Thing.nvs` when `App\Thing` currently resolves to
`vendor/compat/Thing.nvs` changes the answer with no existing file touched. So a unit records the
**ordered list of paths it probed, including the misses**; a probed miss is checked on the same terms
as a file the unit compiled, by the same background check and under the same rate cap, and the trace
folds into the unit's cache key exactly as the target triple does.

A **discovery query** (`rule:programs/implementing`) makes a unit depend on directory *contents*:
adding a module that nothing references must change the generated list. So every directory listed
during the scan — not just the roots, since a directory's `mtime` does not propagate upward — joins
the check, and the sorted list of discovered names hashes into the key, so a listing that changes
without changing the discovered set recompiles nothing.

**No new directive.** Both ride the existing revalidation directives, which are `System`-class
(`rule:config/opcache-revalidation-is-system-class`): bounded at N ⁄ `revalidate_freq` stats per
window for N listed directories — tens, not thousands — spent by the background check of
`rule:config/an-edit-reaches-the-next-request-without-a-restart` and never by a request. For a
compiled build and the wasm target the question does not arise; resolution happens once, at build
time.

**The `autoload` half is on disk; the discovery query's is not.** The resolver records the trace —
every path probed, in order, misses included (`nvs_hir::autoload::ProbeTrace`) — and the in-memory
unit table keys on its digest beside the content hash and the environment, then re-asks those paths
under the gate the content `stat` rides, so a file written where one of them missed sends that unit
to a compile (`nvs_cli::script`). That check still runs inside a resolve, and not at all under
`validate = "never"`. A discovery query's listed directories are not collected at all, and nothing
hashes the discovered names.
