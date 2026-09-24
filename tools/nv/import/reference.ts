// The reference chapters' legacy home: the front matter of each `docs/reference/<path>.md`. It becomes
// `data/reference/<path>.json`. The front matter's `id` is the record's `slug`, because the record's
// own id is its path, and `keywords` is one line split on `, `. A page with no front matter, such as
// the tree's README, is not a chapter and is passed over.

import { referenceChapter } from "../schema/reference.ts";
import { frontMatter, text, walk, type Importer, type ImportResult } from "./lib.ts";

const DIR = "docs/reference";

export const reference: Importer = {
  name: "reference",
  read(root) {
    const out: ImportResult = { records: [], unread: [], files: 0 };
    for (const path of walk(root, DIR, ".md")) {
      const fm = frontMatter(text(root, path));
      if (!fm) continue;
      out.files++;
      const f = fm.fields;
      const extra = [...f.keys()].filter((k) => !["id", "title", "summary", "keywords"].includes(k));
      if (extra.length > 0) out.unread.push({ path, reason: `its front matter carries ${extra.join(", ")}, which no record holds` });
      const value: Record<string, unknown> = {
        summary: f.get("summary") ?? "",
        keywords: (f.get("keywords") ?? "").split(", ").filter((k) => k !== ""),
      };
      if (f.has("id")) value.slug = f.get("id");
      if (f.has("title")) value.title = f.get("title");
      out.records.push({ type: referenceChapter, id: path.slice(DIR.length + 1, -".md".length), value, from: path });
    }
    return out;
  },
};
