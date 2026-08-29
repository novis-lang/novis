/** Typed access to the generated Core reference data (src/data/core.json). */
import data from '../data/core.json'

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
  fromOverride?: boolean
  notesHtml?: string
  doc?: CoreMemberDoc
}

export interface CoreClass {
  id: string
  name: string
  section: string
  summary: string
  surface: string
  adrs: string[]
  enums: { name: string; cases: string[]; raw: string }[]
  constants: string[]
  members: CoreMember[]
  unparsed: { memberCell: string; signatureCell: string; section: string }[]
  implemented: boolean
  slug: string
  url: string
}

export const classes = (data as { classes: CoreClass[] }).classes

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

/** `Str.length` -> `Core\Str::length` */
export function fullName(id: string): string {
  const found = memberIndex.get(id)
  if (!found) return id
  return `${found.cls.name}::${found.member.name}`
}
