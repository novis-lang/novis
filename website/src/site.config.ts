/**
 * The site's identity, in one place.
 *
 * Everything that names the project, its domain, its file extension or its
 * binary reads from here. Nothing else in `website/` hardcodes any of them, so
 * a rename or a domain move is an edit to this file and nothing more.
 *
 * Values that differ per deployment (the base path a version is served under,
 * the version label shown in the switcher) come from the environment so that CI
 * can build the same tree once per ref. Defaults are the local-development
 * answers, so `python site.py dev` needs no environment at all.
 */

/** Where the production site lives. No trailing slash. */
export const SITE_URL = process.env.SITE_URL ?? 'https://novis-lang.org';

/**
 * The path prefix this build is served under, with a leading and trailing
 * slash. `/` for the default (latest stable) build; `/main/` or `/v0.3/` for a
 * version build. CI sets this per ref; see `website/README.md`, § Versioning.
 */
export const SITE_BASE = process.env.SITE_BASE ?? '/';

/**
 * The label this build answers to in the version switcher. `main` locally,
 * because a working tree is the main line until a tag says otherwise.
 */
export const SITE_VERSION = process.env.SITE_VERSION ?? 'main';

/**
 * Where the source lives. Mirrored ADRs link back here for the repository files
 * that are deliberately *not* published (the spec tree, the plan, `crates/`,
 * anything under `docs/agent/`).
 *
 * TODO(owner): point this at the public GitHub repository once it exists. The
 * current `origin` is a private Gitea remote, which a public reader cannot
 * follow. Nothing else needs changing when you do.
 */
export const REPO_URL = process.env.REPO_URL ?? 'https://github.com/BrainFooLong/novis';

/** The branch mirrored links resolve against. */
export const REPO_REF = process.env.REPO_REF ?? 'main';

export const site = {
  /** Display name. The project renamed from MWL to Novis; `nvs` is the token spelling. */
  name: 'Novis',
  /** Token spelling — crates, binary, language id in code fences. */
  token: 'nvs',
  /** Source file extension, with the dot. */
  extension: '.nvs',
  /** The compiler/runtime binary on `PATH`. */
  binary: 'nvs',
  /** The script open tag. */
  openTag: '<?nvs',

  /**
   * One sentence, on the tab title and in search results.
   *
   * Bound by ADR 0080: the pitch is isolation, qualifiers and uncoloured
   * suspension — never "faster than PHP" — and no document may imply PHP
   * compatibility. Read that ADR before changing this line.
   */
  tagline: 'A memory-safe, JIT-compiled language for running code you did not write.',

  url: SITE_URL,
  base: SITE_BASE,
  version: SITE_VERSION,
  repo: REPO_URL,
  repoRef: REPO_REF,

  /** Pre-alpha honesty. Rendered as a site-wide banner; see `StatusBanner.astro`. */
  status: {
    stage: 'pre-alpha',
    milestone: 'M4',
    /** Kept in sync from docs/implementation-plan.md by `site.py sync`. */
    line: 'Novis does not run yet. This site documents a language under construction.',
  },
} as const;

/** A repository file that is not published, linked back to the source host. */
export function repoFile(pathFromRepoRoot: string): string {
  const clean = pathFromRepoRoot.replace(/^\/+/, '');
  return `${REPO_URL}/blob/${REPO_REF}/${clean}`;
}
