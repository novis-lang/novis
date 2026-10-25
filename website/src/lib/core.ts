/** Typed access to the generated Core reference data (src/data/core.json). */
import data from '../data/core.json'
import { withBase } from './base'
import { pages as refPages } from './reference'

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

/** A shape key documented by the registry (`rule:core-api/reference-card`). */
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
 * Registry-carried documentation (`rule:core-api/reference-card`): present only for members the
 * `nvs` binary documents via `nvs meta --json`. Every field is optional, and a
 * page leaves out what the registry lacks.
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
  slug: string
  url: string
  /** The member's feature path in the proofs roster, which is its directory under `docs/examples/`. */
  examples: string
  doc?: CoreMemberDoc
}

/**
 * An enum's registry card (`rule:core-api/reference-card`): its short description and, when any
 * case is documented, one entry per case in the registry's order with its
 * description or '' for a case not written yet.
 */
export interface CoreEnumDoc {
  shortHtml?: string
  cases?: { name: string; descHtml: string }[]
}

export interface CoreClass {
  id: string
  name: string
  /** The class card's one sentence, as Markdown. */
  summary: string
  /** The summary and the class's introduction under `docs/reference/`, as HTML. */
  summaryHtml: string
  enums: { name: string; cases: string[]; raw: string; doc?: CoreEnumDoc }[]
  /** The shape types the class declares, by the name its signatures use: `Info` and `{ format: Format, … }`. */
  types: { name: string; type: string }[]
  constants: { name: string; descHtml: string }[]
  members: CoreMember[]
  slug: string
  url: string
}

export const classes = (data as { classes: CoreClass[] }).classes

// The generated data holds root-absolute URLs — as `url`/`slug` fields and
// inside pre-rendered HTML (`summaryHtml`, `descHtml`, …). The
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
 * Where a type name in a signature links, and the one home for that mapping.
 *
 * A name is read the way the compiler reads it inside the class whose
 * signature it is in: an enum or shape type that class declares, a class in
 * the namespace the class opens, a class or type in the class's namespace and
 * in each namespace above it, the name as written, and the name under `Core\`.
 * Then comes the one type of that name that a class in the same namespace
 * declares. So `Standard` in `Novis\Pdf\Document` is
 * `Novis\Pdf\Document::Standard`, `Match` in `Core\Regex` is
 * `Core\Regex\Match`, and `Time\Date` in a Core class is `Core\Time\Date`.
 *
 * A class goes to its page, an enum, exception or interface to its Types
 * page, and a shape type to its own heading on its class page. An enum
 * with no Types page goes to its class page's `#enums` section. A name that is
 * the last segment of exactly one of these is the last resort, and a type
 * built into the language goes to its row in `LANGUAGE_TYPES`. A name in
 * `NO_PAGE` has no link, and any other name fails the build, so a signature
 * never links to a page that does not describe it.
 */
/** The id of a shape type's heading on its class page: `type-info` for `Info`. */
export const shapeAnchor = (name: string) => `type-${name.toLowerCase()}`

/**
 * Each type built into the language, to the section of a Syntax or concept
 * page that explains it. A path has no base: `typeLink` adds it. Each anchor is
 * a `FeatureSection` id or a `##` heading on that page, and
 * tools/nv/test/core-type-links.test.ts fails when the page or the anchor is gone.
 */
const LANGUAGE_TYPES = new Map([
  ['string', '/syntax/values/strings/#text-and-bytes'],
  ['bytes', '/syntax/values/strings/#text-and-bytes'],
  ['int', '/syntax/values/numbers/#the-number-types'],
  ['uint', '/syntax/values/numbers/#the-number-types'],
  ['float', '/syntax/values/numbers/#the-number-types'],
  ['decimal', '/syntax/values/numbers/#the-number-types'],
  ['bool', '/syntax/values/bool-and-null/'],
  ['null', '/syntax/values/bool-and-null/'],
  ['mixed', '/syntax/values/union-types/#mixed'],
  ['void', '/syntax/functions/declaring/#void-never-self-static'],
  ['never', '/syntax/functions/declaring/#void-never-self-static'],
  ['self', '/syntax/functions/declaring/#void-never-self-static'],
  ['static', '/syntax/functions/declaring/#void-never-self-static'],
  ['callable', '/syntax/functions/anonymous-functions/'],
  ['array', '/syntax/values/arrays/'],
  ['object', '/syntax/classes/objects/#the-object-type'],
  ['Iterable', '/syntax/control-flow/iterators/#iterable-and-iterator'],
  ['Iterator', '/syntax/control-flow/iterators/#iterable-and-iterator'],
  ['property', '/in-depth/concepts/checked-property-names/#a-property-name'],
  ['propertyPath', '/in-depth/concepts/checked-property-names/#a-path-through-objects'],
])

/** Every linkable name, fully qualified (`Core\Str`, `Core\Arr::Order`, `Throwable`), to its page. */
const pageOf = new Map<string, string>()
for (const cls of classes) pageOf.set(cls.name, cls.url)
for (const page of refPages) if (page.area === 'types' && page.intro) pageOf.set(page.intro.id, page.url)
for (const cls of classes) for (const t of cls.types) pageOf.set(`${cls.name}::${t.name}`, `${cls.url}#${shapeAnchor(t.name)}`)

/** The fully qualified names `name` may mean inside `owner`, most specific first. */
function candidates(name: string, owner: string | undefined): string[] {
  const out: string[] = []
  if (owner) {
    out.push(`${owner}::${name}`, `${owner}\\${name}`)
    for (let ns = owner; ns.includes('\\'); ) {
      ns = ns.slice(0, ns.lastIndexOf('\\'))
      out.push(`${ns}\\${name}`)
    }
  }
  out.push(name, `Core\\${name}`)
  return out
}

const lookup = (name: string, owner: string | undefined) =>
  candidates(name, owner).map((full) => pageOf.get(full)).find((url) => url !== undefined)

// An enum the class page lists, which has no Types page of its own.
for (const cls of classes) {
  for (const e of cls.enums) if (!lookup(e.name, cls.name)) pageOf.set(`${cls.name}::${e.name}`, `${cls.url}#enums`)
}

const add = (index: Map<string, Set<string>>, key: string, url: string) => {
  if (!index.has(key)) index.set(key, new Set())
  index.get(key)!.add(url)
}
/** `Novis\Image|Level` to the page of every enum or shape type named `Level` that a class in `Novis\Image` declares. */
const declaredIn = new Map<string, Set<string>>()
const byLastSegment = new Map<string, Set<string>>()
for (const [full, url] of pageOf) {
  const [cls, member] = full.split('::')
  if (member && cls.includes('\\')) add(declaredIn, `${cls.slice(0, cls.lastIndexOf('\\'))}|${member}`, url)
  add(byLastSegment, full.split(/::|\\/).pop() ?? full, url)
}

/** The one page of a type named `name` that a class in `owner`'s namespace, or one above it, declares. */
function declaredNearby(name: string, owner: string | undefined): string | undefined {
  for (let ns = owner ?? ''; ns.includes('\\'); ) {
    ns = ns.slice(0, ns.lastIndexOf('\\'))
    const urls = declaredIn.get(`${ns}|${name}`)
    if (urls?.size === 1) return [...urls][0]
  }
  return undefined
}

/**
 * Names a signature uses that have no page anywhere, so they are shown without
 * a link, because the binary does not list them. A name is added here only
 * when there is nothing to link it to.
 */
const NO_PAGE = new Set(['Db\\Settings', 'StrongDigest', 'Sink', 'method'])

/**
 * One type name in a signature, split out of the text around it with `split`: `int`, `Time\Date`,
 * `Core\Arr::Order` or the enum case `Core\Hash::Digest::Sha256`.
 */
export const TYPE_NAME = /([A-Za-z_][A-Za-z0-9_\\]*(?:::[A-Za-z_][A-Za-z0-9_]*)*)/

/**
 * The page a type name in a signature of `owner` links to; `null` for a
 * generic parameter or a name in `NO_PAGE`. Throws for any other name with no
 * page.
 */
export function typeLink(name: string, owner?: string): string | null {
  if (/^[A-Z]$/.test(name)) return null
  const found = lookup(name, owner) ?? declaredNearby(name, owner)
  if (found) return found
  // An enum case, `Core\Hash::Digest::Sha256`, links to its enum's page.
  const enumCase = /^(.+::[A-Za-z_][A-Za-z0-9_]*)::[A-Za-z_][A-Za-z0-9_]*$/.exec(name)
  if (enumCase) return typeLink(enumCase[1]!, owner)
  const language = LANGUAGE_TYPES.get(name)
  if (language) return withBase(language)
  const only = byLastSegment.get(name)
  if (only?.size === 1) return [...only][0]
  if (NO_PAGE.has(name)) return null
  throw new Error(`the type ${name} in a signature of ${owner ?? 'a Core class'} has no page: run \`bun nv render --website\`, or give it one`)
}

/** `Str.length` -> `Core\Str::length` */
export function fullName(id: string): string {
  const found = memberIndex.get(id)
  if (!found) return id
  return `${found.cls.name}::${found.member.name}`
}
