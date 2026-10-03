/** Typed access to the generated Core reference data (src/data/core.json). */
import data from '../data/core.json'
import { withBase } from './base'

export interface CoreParam {
  kind: 'param'
  name: string
  type: string
  default: string | null
  optional: boolean
  variadic: boolean
}

export interface CoreOption {
  name: string
  type: string
}

/** A shape key documented by the registry (ADR 0117). */
export interface CoreShapeKeyDoc {
  key: string
  type: string
  descHtml: string
}

export interface CoreParamDoc {
  name: string
  descHtml: string
  shape?: CoreShapeKeyDoc[]
}

/**
 * Registry-carried documentation (ADR 0117): present only for members the
 * `nvs` binary documents via `nvs meta --json`. Every field is optional; a
 * field the registry lacks falls back to the spec-derived default.
 */
export interface CoreMemberDoc {
  shortHtml?: string
  params?: CoreParamDoc[]
  returnHtml?: string
  errors?: { error: string; descHtml: string }[]
}

export interface CoreMember {
  id: string
  name: string
  static: boolean
  receiver: string | null
  generics: string | null
  signature: string
  params: CoreParam[]
  options: CoreOption[]
  returnType: string
  replaces: string
  notes: string
  qualifier: string
  section: string
  implemented: boolean
  slug: string
  url: string
  /** The member's feature path in the proofs roster, which is its directory under `docs/examples/`. */
  examples: string
  fromOverride?: boolean
  notesHtml?: string
  doc?: CoreMemberDoc
}

/**
 * An enum's registry card (ADR 0117): its short description and, when any
 * case is documented, one entry per spec-declared case in spec order with the
 * registry's description or '' for a case not written yet.
 */
export interface CoreEnumDoc {
  shortHtml?: string
  cases?: { name: string; descHtml: string }[]
}

export interface CoreClass {
  id: string
  name: string
  section: string
  summary: string
  surface: string
  adrs: string[]
  enums: { name: string; cases: string[]; raw: string; doc?: CoreEnumDoc }[]
  constants: { name: string; descHtml: string }[]
  members: CoreMember[]
  unparsed: { memberCell: string; signatureCell: string; section: string }[]
  implemented: boolean
  slug: string
  url: string
}

export const classes = (data as { classes: CoreClass[] }).classes

// The generated data holds root-absolute URLs — as `url`/`slug` fields and
// inside pre-rendered HTML (`summary`, `descHtml`, `notesHtml`, …). The
// site's `base` path is prefixed once here so everything downstream
// (indexes included) uses the deployed URL verbatim. Attribute values in the
// HTML strings are unescaped, while code samples inside them are escaped
// (`&quot;`), so the pattern cannot touch example code.
const urlAttr = /(href|src)="(\/[^/"][^"]*|\/)"/g
function prefixHtmlUrls(value: unknown): void {
  if (Array.isArray(value)) {
    value.forEach(prefixHtmlUrls)
  } else if (value && typeof value === 'object') {
    for (const [key, entry] of Object.entries(value)) {
      if (typeof entry === 'string') {
        ;(value as Record<string, unknown>)[key] = entry.replace(
          urlAttr,
          (_, attr, url) => `${attr}="${withBase(url)}"`
        )
      } else {
        prefixHtmlUrls(entry)
      }
    }
  }
}
prefixHtmlUrls(classes)
for (const cls of classes) {
  cls.url = withBase(cls.url)
  for (const member of cls.members) member.url = withBase(member.url)
}

const memberIndex = new Map<string, { cls: CoreClass; member: CoreMember }>()
const classIndex = new Map<string, CoreClass>()
for (const cls of classes) {
  classIndex.set(cls.id, cls)
  for (const member of cls.members) {
    memberIndex.set(member.id, { cls, member })
  }
}

export function findClass(id: string): CoreClass | undefined {
  return classIndex.get(id)
}

export function findMember(id: string): { cls: CoreClass; member: CoreMember } | undefined {
  return memberIndex.get(id)
}

/**
 * Where a type name links from a signature. Placeholder targets until the
 * pages exist: the built-in scalars and any name nothing else claims go to the
 * language's types page, an enum the spec attaches to a class goes to that
 * class page's `#enums` section, and a Core class goes to its own page.
 * Change the two constants below when the real pages land — this is the one
 * home for the mapping.
 */
export const TYPES_PAGE = withBase('/syntax/')
export const ENUMS_ANCHOR = '#enums'

const BUILTIN_TYPES = new Set([
  'string', 'int', 'uint', 'float', 'decimal', 'bool', 'bytes', 'mixed', 'void', 'null', 'never',
  'callable', 'array', 'object', 'Iterable', 'Iterator', 'self', 'static',
])

const typeIndex = new Map<string, string>()
for (const cls of classes) {
  for (const e of cls.enums) typeIndex.set(e.name, `${cls.url}${ENUMS_ANCHOR}`)
}
const shortNames = new Map<string, string[]>()
for (const cls of classes) {
  const local = cls.name.replace(/^Core\\/, '')
  typeIndex.set(local, cls.url)
  const last = local.split('\\').pop() ?? local
  shortNames.set(last, [...(shortNames.get(last) ?? []), cls.url])
}
for (const [last, urls] of shortNames) {
  if (urls.length === 1 && !typeIndex.has(last)) typeIndex.set(last, urls[0])
}

/** The page a type name in a signature links to; `null` for a generic parameter. */
export function typeLink(name: string): string | null {
  if (/^[A-Z]$/.test(name)) return null
  if (BUILTIN_TYPES.has(name)) return `${TYPES_PAGE}#${name}`
  return typeIndex.get(name) ?? `${TYPES_PAGE}#${name.replace(/\\/g, '-')}`
}

/** `Str.length` -> `Core\Str::length` */
export function fullName(id: string): string {
  const found = memberIndex.get(id)
  if (!found) return id
  return `${found.cls.name}::${found.member.name}`
}
