// The typed readers `looper.toml` and a goal's front matter share, with serde's messages. Each reader
// takes a value `Bun.TOML` or `Bun.YAML` parsed and returns it typed, or throws a `Refusal` at the path
// of keys and indexes it was found on. The caller turns the path into a line of the file.

/** The string as Rust's `{:?}` writes it, which JSON's quoting matches for every string a file holds. */
export function debug(s: string): string {
  return JSON.stringify(s);
}

export type Seg = string | number;

/** What of a refused path the message shows: its last key, its value, or the whole table it names. */
export type Point = "key" | "value" | "table";

/** A value refused at `at`, the path of keys and indexes from the root. */
export class Refusal {
  constructor(
    readonly at: Seg[],
    readonly message: string,
    readonly point: Point,
  ) {}
}

export interface Reader<T> {
  expects: string;
  read(v: unknown, at: Seg[]): T;
}

export function isMap(v: unknown): v is Record<string, unknown> {
  return typeof v === "object" && v !== null && !Array.isArray(v) && Object.getPrototypeOf(v) === Object.prototype;
}

/** What serde calls the value it found. */
export function found(v: unknown): string {
  if (v === null || v === undefined) return "unit value";
  if (typeof v === "string") return `string ${debug(v)}`;
  if (typeof v === "boolean") return `boolean \`${v}\``;
  if (typeof v === "number") return Number.isInteger(v) ? `integer \`${v}\`` : `floating point \`${v}\``;
  if (Array.isArray(v)) return "sequence";
  return "map";
}

export function wrongType(v: unknown, expects: string, at: Seg[]): never {
  throw new Refusal(at, `invalid type: ${found(v)}, expected ${expects}`, "value");
}

/** The names as serde lists what it expected. */
export function oneOf(names: readonly string[]): string {
  const ticked = names.map((n) => `\`${n}\``);
  if (ticked.length === 1) return ticked[0] as string;
  if (ticked.length === 2) return `${ticked[0]} or ${ticked[1]}`;
  return `one of ${ticked.join(", ")}`;
}

export const str: Reader<string> = {
  expects: "a string",
  read: (v, at) => (typeof v === "string" ? v : wrongType(v, "a string", at)),
};

export const bool: Reader<boolean> = {
  expects: "a boolean",
  read: (v, at) => (typeof v === "boolean" ? v : wrongType(v, "a boolean", at)),
};

export function uint(name: string, max: number): Reader<number> {
  return {
    expects: name,
    read(v, at) {
      if (typeof v !== "number" || !Number.isInteger(v)) return wrongType(v, name, at);
      if (v < 0 || v > max) throw new Refusal(at, `invalid value: integer \`${v}\`, expected ${name}`, "value");
      return v;
    },
  };
}

export const u16 = uint("u16", 0xffff);
export const u32 = uint("u32", 0xffff_ffff);
export const u64 = uint("u64", Number.MAX_SAFE_INTEGER);
export const usize = uint("usize", Number.MAX_SAFE_INTEGER);

export function list<T>(item: Reader<T>): Reader<T[]> {
  return {
    expects: "a sequence",
    read: (v, at) => (Array.isArray(v) ? v.map((x, i) => item.read(x, [...at, i])) : wrongType(v, "a sequence", at)),
  };
}

export const strings = list(str);
export const argvs = list(strings);

/** A map of strings to `value`'s type. */
export function mapOf<T>(value: Reader<T>): Reader<Record<string, T>> {
  return {
    expects: "a map",
    read(v, at) {
      if (!isMap(v)) return wrongType(v, "a map", at);
      const out: Record<string, T> = {};
      for (const [k, x] of Object.entries(v)) out[k] = value.read(x, [...at, k]);
      return out;
    },
  };
}

export const strMap = mapOf(str);

export function variants<T extends string>(names: readonly T[]): Reader<T> {
  return {
    expects: "variant identifier",
    read(v, at) {
      if (typeof v !== "string") return wrongType(v, "variant identifier", at);
      if (!names.includes(v as T)) throw new Refusal(at, `unknown variant \`${v}\`, expected ${oneOf(names)}`, "value");
      return v as T;
    },
  };
}

/**
 * A struct: each key in the file's order goes to its field's reader, a key no field has is refused, and
 * a key left out keeps `make`'s value. A key in `required` must be in the file.
 */
export function table<T extends object>(
  name: string,
  fields: Partial<Record<keyof T, Reader<unknown>>>,
  make: () => T,
  required: (keyof T & string)[] = [],
): Reader<T> {
  const names = Object.keys(fields);
  return {
    expects: `struct ${name}`,
    read(v, at) {
      if (!isMap(v)) return wrongType(v, `struct ${name}`, at);
      const out = make() as Record<string, unknown>;
      for (const [k, x] of Object.entries(v)) {
        const field = (fields as Record<string, Reader<unknown> | undefined>)[k];
        if (field === undefined) {
          const expected = names.length === 0 ? "there are no fields" : `expected ${oneOf(names)}`;
          throw new Refusal([...at, k], `unknown field \`${k}\`, ${expected}`, "key");
        }
        out[k] = field.read(x, [...at, k]);
      }
      const missing = required.find((k) => !(k in v));
      if (missing !== undefined) throw new Refusal(at, `missing field \`${missing}\``, "table");
      return out as T;
    },
  };
}
