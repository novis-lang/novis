// The record schemas: one JSON Schema file per record type under `data/schema/`, named for the type,
// so an editor or the website can check a record without running `bun nv`. Each is the type's
// declaration under `tools/nv/schema/` walked by `jsonSchema`, so it changes only when a declaration
// does. A record type's references (`s.ref`) are plain strings here, tagged `x-references`; only the
// index checks one against the records it names. `recordFiles` skips this directory, so the index
// never reads a schema as a record.

import { SCHEMA_DIR } from "../lib/store.ts";
import { MARKER, type Output, type Renderer } from "../lib/render.ts";
import { RECORDS } from "../schema/index.ts";

const DIALECT = "https://json-schema.org/draft/2020-12/schema";

/** One schema file per record type, in `RECORDS` order. */
export function renderRecordSchemas(): Output[] {
  return RECORDS.map((t) => ({
    path: `${SCHEMA_DIR}/${t.name}.schema.json`,
    text: JSON.stringify({ $comment: MARKER, $schema: DIALECT, title: t.name, ...t.schema.jsonSchema() }, null, 2) + "\n",
  }));
}

export const recordSchemas: Renderer = {
  name: "record-schemas",
  render: () => renderRecordSchemas(),
  owns: { dir: SCHEMA_DIR, keep: [] },
};
