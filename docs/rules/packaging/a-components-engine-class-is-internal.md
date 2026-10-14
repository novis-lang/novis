A built-in component's manifest class — `Novis\Image\Codec`, `Novis\Intl\Icu`, `Novis\Pdf\Engine`,
`Novis\Spreadsheet\Engine` — is called only by that component's own Novis source. A call of one of its
methods, or a reference to one, anywhere else is `E0856`, and the error's help is the class's card,
which names the public class a program uses instead.

The engine is plumbing. Its methods take the records the component's Novis classes build, in the
shape the WIT interface needs, and every one of them is called by those classes, so a program loses
no capability and has one way to do each job. The engine stays in the binary, is loaded and typed
like every manifest class (`rule:packaging/extension-calls-are-statically-typed`), and keeps its
`nvs meta --json` entry, which carries `"internal": true`. So does a manifest enum or a source
class's type alias that no public member's signature of the component names, directly or through a
type alias such a signature uses: only the engine takes it. Neither is a feature, so neither is on the proofs roster
(`rule:testing/feature-proofs`), in `data/proofs/policy.json` or on the website.

A file is the component's own when `nvs_hir`'s graph walk added it from the component's `nvs.source`
section: it carries the name `<class>:<path>` and no path on disk, so no file a program writes or
`require`s is one. The check is in `nvs_types::expr::calls`, where a static call resolves its class,
and reads `ClassSignature::engine`, which `nvs_types::ext_lib::seed` fills from every manifest the
binary embeds. An `[[extension]]` entry's class is not an engine: the entry is the program's own
choice and its class is its public surface.

- **Memory:** one card string per built-in component, held by each check's signature table.

**Declined:**
- **A separate list of public classes per engine, for the error to name.** The card already names
  them, and a second list would be a second home for the same fact.
- **Hiding the engine from name resolution.** A program would then get "unknown class" for a class
  the binary does have, and no pointer to the class it wanted.
