// The schema builder. One declaration gives three things: the static type (`Infer<typeof s>`), the
// runtime check (`validate`), and a JSON Schema (`jsonSchema`), which the index hashes into its layout
// fingerprint. A fourth falls out of the same walk: `ordered`, which puts an object's keys in
// declaration order and is how the writer makes every record's key order the schema's.
//
// A reference is declared with `s.ref("<record type>")`. It is an id at run time and a foreign key in
// the index, which is the only place it is checked against the records it names.

export interface Issue {
  /** Where in the value, as a JSON pointer-ish path: `checks[2].id`. Empty for the value itself. */
  at: string;
  message: string;
}

export type JsonSchema = { [key: string]: unknown };

export abstract class Schema<T> {
  /** Never set: it carries `T` for `Infer`. */
  declare readonly _type: T;
  readonly isOptional: boolean = false;
  description: string | undefined;

  abstract check(value: unknown, at: string, issues: Issue[]): void;
  protected abstract schema(): JsonSchema;

  /** Every way `value` fails this schema. Empty when it is a `T`. */
  validate(value: unknown): Issue[] {
    const issues: Issue[] = [];
    this.check(value, "", issues);
    return issues;
  }

  is(value: unknown): value is T {
    return this.validate(value).length === 0;
  }

  /** `value` as a `T`, or an error naming every issue. */
  parse(value: unknown): T {
    const issues = this.validate(value);
    if (issues.length > 0) throw new SchemaError(issues);
    return value as T;
  }

  jsonSchema(): JsonSchema {
    const out = this.schema();
    return this.description === undefined ? out : { description: this.description, ...out };
  }

  /** `value` rebuilt with every object's keys in declaration order. `value` must already be valid. */
  ordered(value: T): T {
    return value;
  }

  /** The same schema with a description, which the JSON Schema carries. */
  describe(text: string): this {
    const copy = Object.assign(Object.create(Object.getPrototypeOf(this)), this) as this;
    copy.description = text;
    return copy;
  }
}

export class SchemaError extends Error {
  constructor(readonly issues: Issue[]) {
    super(issues.map((i) => `${i.at || "(value)"}: ${i.message}`).join("\n"));
  }
}

function describeValue(v: unknown): string {
  if (v === null) return "null";
  if (Array.isArray(v)) return "an array";
  return typeof v === "object" ? "an object" : `${typeof v} ${JSON.stringify(v)}`;
}

function join(at: string, key: string | number): string {
  if (typeof key === "number") return `${at}[${key}]`;
  return at === "" ? key : `${at}.${key}`;
}

export class StringSchema extends Schema<string> {
  constructor(readonly pattern?: RegExp, readonly minLength = 0) {
    super();
  }
  check(value: unknown, at: string, issues: Issue[]): void {
    if (typeof value !== "string") {
      issues.push({ at, message: `wants a string, found ${describeValue(value)}` });
    } else if (value.length < this.minLength) {
      issues.push({ at, message: `wants at least ${this.minLength} character(s)` });
    } else if (this.pattern && !this.pattern.test(value)) {
      issues.push({ at, message: `${JSON.stringify(value)} does not match ${this.pattern.source}` });
    }
  }
  protected schema(): JsonSchema {
    const out: JsonSchema = { type: "string" };
    if (this.pattern) out.pattern = this.pattern.source;
    if (this.minLength > 0) out.minLength = this.minLength;
    return out;
  }
}

export class NumberSchema extends Schema<number> {
  constructor(readonly integer: boolean) {
    super();
  }
  check(value: unknown, at: string, issues: Issue[]): void {
    if (typeof value !== "number" || !Number.isFinite(value)) {
      issues.push({ at, message: `wants a number, found ${describeValue(value)}` });
    } else if (this.integer && !Number.isInteger(value)) {
      issues.push({ at, message: `wants a whole number, found ${value}` });
    }
  }
  protected schema(): JsonSchema {
    return { type: this.integer ? "integer" : "number" };
  }
}

export class BooleanSchema extends Schema<boolean> {
  check(value: unknown, at: string, issues: Issue[]): void {
    if (typeof value !== "boolean") issues.push({ at, message: `wants a boolean, found ${describeValue(value)}` });
  }
  protected schema(): JsonSchema {
    return { type: "boolean" };
  }
}

export class EnumSchema<const V extends string> extends Schema<V> {
  constructor(readonly values: readonly V[]) {
    super();
  }
  check(value: unknown, at: string, issues: Issue[]): void {
    if (!(this.values as readonly unknown[]).includes(value)) {
      issues.push({ at, message: `wants one of ${this.values.join(", ")}, found ${describeValue(value)}` });
    }
  }
  protected schema(): JsonSchema {
    return { enum: [...this.values] };
  }
}

/** An id naming a record of type `target`. The index declares it as a foreign key. */
export class RefSchema extends StringSchema {
  constructor(readonly target: string) {
    super(undefined, 1);
  }
  protected override schema(): JsonSchema {
    return { type: "string", minLength: 1, "x-references": this.target };
  }
}

export class ArraySchema<T> extends Schema<T[]> {
  constructor(readonly item: Schema<T>) {
    super();
  }
  check(value: unknown, at: string, issues: Issue[]): void {
    if (!Array.isArray(value)) {
      issues.push({ at, message: `wants an array, found ${describeValue(value)}` });
      return;
    }
    value.forEach((v, i) => this.item.check(v, join(at, i), issues));
  }
  protected schema(): JsonSchema {
    return { type: "array", items: this.item.jsonSchema() };
  }
  override ordered(value: T[]): T[] {
    return value.map((v) => this.item.ordered(v));
  }
}

/** An object whose keys are data rather than fields. The writer sorts its keys. */
export class MapSchema<T> extends Schema<Record<string, T>> {
  constructor(readonly value: Schema<T>) {
    super();
  }
  check(value: unknown, at: string, issues: Issue[]): void {
    if (typeof value !== "object" || value === null || Array.isArray(value)) {
      issues.push({ at, message: `wants an object, found ${describeValue(value)}` });
      return;
    }
    for (const [k, v] of Object.entries(value)) this.value.check(v, join(at, k), issues);
  }
  protected schema(): JsonSchema {
    return { type: "object", additionalProperties: this.value.jsonSchema() };
  }
  override ordered(value: Record<string, T>): Record<string, T> {
    const out: Record<string, T> = {};
    for (const k of Object.keys(value).sort()) out[k] = this.value.ordered(value[k] as T);
    return out;
  }
}

export class NullableSchema<T> extends Schema<T | null> {
  constructor(readonly inner: Schema<T>) {
    super();
  }
  check(value: unknown, at: string, issues: Issue[]): void {
    if (value !== null) this.inner.check(value, at, issues);
  }
  protected schema(): JsonSchema {
    return { anyOf: [this.inner.jsonSchema(), { type: "null" }] };
  }
  override ordered(value: T | null): T | null {
    return value === null ? null : this.inner.ordered(value);
  }
}

/** A field that may be absent. It is never `undefined`: a record either has the key or does not. */
export class OptionalSchema<T> extends Schema<T> {
  override readonly isOptional = true as const;
  constructor(readonly inner: Schema<T>) {
    super();
  }
  check(value: unknown, at: string, issues: Issue[]): void {
    this.inner.check(value, at, issues);
  }
  protected schema(): JsonSchema {
    return this.inner.jsonSchema();
  }
  override ordered(value: T): T {
    return this.inner.ordered(value);
  }
}

export type Shape = { [key: string]: Schema<any> };
type OptionalKeys<S extends Shape> = { [K in keyof S]: S[K] extends { isOptional: true } ? K : never }[keyof S];
type Pretty<T> = { [K in keyof T]: T[K] } & {};
export type InferShape<S extends Shape> = Pretty<
  { [K in Exclude<keyof S, OptionalKeys<S>>]: Infer<S[K]> } & { [K in OptionalKeys<S>]?: Infer<S[K]> }
>;

export class ObjectSchema<S extends Shape> extends Schema<InferShape<S>> {
  constructor(readonly shape: S) {
    super();
  }
  check(value: unknown, at: string, issues: Issue[]): void {
    if (typeof value !== "object" || value === null || Array.isArray(value)) {
      issues.push({ at, message: `wants an object, found ${describeValue(value)}` });
      return;
    }
    const obj = value as Record<string, unknown>;
    for (const [k, field] of Object.entries(this.shape)) {
      if (!(k in obj)) {
        if (!field.isOptional) issues.push({ at: join(at, k), message: "is missing" });
      } else {
        field.check(obj[k], join(at, k), issues);
      }
    }
    for (const k of Object.keys(obj)) {
      if (!(k in this.shape)) issues.push({ at: join(at, k), message: "is not a field of this record" });
    }
  }
  protected schema(): JsonSchema {
    const properties: JsonSchema = {};
    const required: string[] = [];
    for (const [k, field] of Object.entries(this.shape)) {
      properties[k] = field.jsonSchema();
      if (!field.isOptional) required.push(k);
    }
    return { type: "object", properties, required, additionalProperties: false };
  }
  override ordered(value: InferShape<S>): InferShape<S> {
    const obj = value as Record<string, unknown>;
    const out: Record<string, unknown> = {};
    for (const [k, field] of Object.entries(this.shape)) {
      if (k in obj) out[k] = field.ordered(obj[k]);
    }
    return out as InferShape<S>;
  }
}

export type Infer<S> = S extends Schema<infer T> ? T : never;

const SLUG = /^[a-z0-9][a-z0-9-]*$/;

/** The builder. Every record type under `tools/nv/schema/` is written with it. */
export const s = {
  string: (pattern?: RegExp) => new StringSchema(pattern),
  /** A lower-case, hyphenated name: the shape every slug id in this tree has. */
  slug: () => new StringSchema(SLUG),
  number: () => new NumberSchema(false),
  int: () => new NumberSchema(true),
  boolean: () => new BooleanSchema(),
  enum: <const V extends string>(...values: V[]) => new EnumSchema<V>(values),
  ref: (target: string) => new RefSchema(target),
  array: <T>(item: Schema<T>) => new ArraySchema(item),
  map: <T>(value: Schema<T>) => new MapSchema(value),
  nullable: <T>(inner: Schema<T>) => new NullableSchema(inner),
  optional: <T>(inner: Schema<T>) => new OptionalSchema(inner),
  object: <S extends Shape>(shape: S) => new ObjectSchema(shape),
};

/**
 * One kind of record: its schema, where its files live under `data/`, and what the index adds for it.
 *
 * `dir` is repo-relative under `data/`. A `single` type is the one file `data/<dir>.json` and its id is
 * its name; any other type is every `.json` file under `data/<dir>/` that `idOf` accepts, and its id is
 * the path under that directory without `.json`, so a nested file's id keeps its slash.
 */
export interface RecordType<T = unknown> {
  /** The entity's name, which is also its table in the index: lower case and underscores. */
  name: string;
  dir: string;
  single?: boolean;
  /** How a file of this type's name ends after its id: `.json` unless set. Its `idOf` must agree. */
  suffix?: string;
  schema: Schema<T>;
  /** The id of the file at `sub` (relative to `data/<dir>/`), or null when it is not this type's. */
  idOf?: (sub: string) => string | null;
  /** SQL views the index creates for this type, by name. */
  views?: Record<string, string>;
  /**
   * Invariants the index cannot declare as a constraint. Each query returns the rows that break it,
   * with a `path` column and a `detail` column; `nv check` prints one finding per row.
   */
  checks?: { name: string; sql: string }[];
}

export function defineRecord<T>(type: RecordType<T>): RecordType<T> {
  if (!/^[a-z][a-z0-9_]*$/.test(type.name)) throw new Error(`record type name ${type.name}: lower case and underscores only`);
  return type;
}

/** The default `idOf`: a `.json` file whose name has no other dot. */
export function plainIdOf(sub: string): string | null {
  if (!sub.endsWith(".json")) return null;
  const id = sub.slice(0, -".json".length);
  return id.includes(".") ? null : id;
}
