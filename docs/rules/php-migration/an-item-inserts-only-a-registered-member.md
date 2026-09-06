A PHP-name item may insert text only when its destination resolves — a `Core` member that
`crates/nvs-stdlib/src/registry.rs` actually holds, the same table `nvs-types` seeds its signatures
from, so *resolves* means here what it means to the checker. A registered destination inserts its
member with the signature the registry holds. A destination the migration table promises and the
registry does not yet hold produces an item that appears, names the milestone it is waiting on, and
inserts nothing; a `dropped` row appears with its reason and rewrite and inserts nothing; an undecided
name appears, says so, and inserts nothing. Three of the four item shapes type nothing at all.

An editor that inserts a call which then fails to resolve is worse than one that offers nothing: the
developer cannot tell the suggestion's mistake from their own. A *name* may come from an audited
table, but the text an editor types on a developer's behalf comes only from something the compiler
can already resolve.

Hiding the unbuilt items instead is refused: a name that vanishes reads as a language that cannot do
the job, where one that names its milestone reads as a language with a schedule.
`rule:php-migration/completion-php-names-setting` is the lever for a developer who wants only the
items that go somewhere.
