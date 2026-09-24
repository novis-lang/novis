// Every record type, in one list. The index builds a table for each, `nv check` checks each, and
// `data/schema/` publishes each one's JSON Schema. A type is declared in a file of its own beside this
// one, with `defineRecord` from `../lib/schema.ts`, and added here.
//
// The list is empty until the importer lands the records it reads out of their legacy homes; a type
// is added in the same change as the importer that first writes its records.

import type { RecordType } from "../lib/schema.ts";

export const RECORDS: RecordType<any>[] = [];
