// The website's Novis grammar: `website/config/novis.tmLanguage.json`, which Expressive Code loads for
// a ```nvs fence, rendered from the editors' grammar at `editors/vscode/syntaxes/nvs.tmLanguage.json`.
// Every rule is the editors' own. One thing differs, and it is the reason this is a render and not a
// copy: a file opens in text mode, and a fence on the site is a snippet with no `<?nvs` in front of it,
// so the top level here is code mode, with the opener and the shebang still tried first so a whole
// file reads the same as in the editor. The `comment` fields are left out, because the site ships the
// grammar to no one who reads them.

import { readFileSync } from "node:fs";
import { join } from "node:path";
import { MARKER, type Output, type Renderer } from "../lib/render.ts";

export const EDITOR_GRAMMAR = "editors/vscode/syntaxes/nvs.tmLanguage.json";
export const WEBSITE_GRAMMAR = "website/config/novis.tmLanguage.json";

function withoutComments(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(withoutComments);
  if (value === null || typeof value !== "object") return value;
  const out: Record<string, unknown> = {};
  for (const [k, v] of Object.entries(value)) if (k !== "comment") out[k] = withoutComments(v);
  return out;
}

/** The editors' grammar with code mode at its top level, under the language ids a fence names. */
export function renderWebsiteGrammar(editor: string): Output {
  const { $schema, name, scopeName, injections, repository } = withoutComments(JSON.parse(editor)) as Record<
    string,
    unknown
  >;
  const grammar = {
    $schema,
    comment: MARKER,
    name,
    aliases: ["novis", "nvs"],
    scopeName,
    patterns: [{ include: "#shebang" }, { include: "#code-mode" }, { include: "#code" }],
    injections,
    repository,
  };
  return { path: WEBSITE_GRAMMAR, text: JSON.stringify(grammar, null, 2) + "\n" };
}

export const websiteGrammar: Renderer = {
  name: "website-grammar",
  render: (root) => [renderWebsiteGrammar(readFileSync(join(root, EDITOR_GRAMMAR), "utf8"))],
};
