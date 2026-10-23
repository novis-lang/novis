/**
 * The released version and the names of its downloads. The version is the workspace version in the
 * root `Cargo.toml`, read at build time, so no page writes a version number by hand. An archive is
 * named `nvs-<version>-<name>` as `bun nv release --package` writes it, and the `.vsix` is the
 * editor extension the same release attaches.
 */
import cargo from '../../../Cargo.toml?raw'

const found = /^\[workspace\.package\][^[]*?^version\s*=\s*"([^"]+)"/m.exec(cargo)
if (!found) throw new Error('release.ts: Cargo.toml has no [workspace.package] version')

/** The released version, such as `0.0.1`. */
export const version: string = found[1]

/** The minor line a `MAJOR.MINOR` Docker tag follows, such as `0.0`. */
export const minor: string = version.split('.').slice(0, 2).join('.')

/** The status every page says. */
export const status = 'Pre-Alpha'

export const repository = 'https://github.com/novis-lang/novis'

/** The release page of this version. */
export const releaseUrl = `${repository}/releases/tag/v${version}`

/** The image every release pushes. */
export const image = 'ghcr.io/novis-lang/novis'

const download = (file: string) => `${repository}/releases/download/v${version}/${file}`

export interface Archive {
  /** The name `release.yml`'s build matrix gives the target. */
  name: string
  os: 'Windows' | 'Linux' | 'macOS'
  label: string
  file: string
  url: string
}

const archive = (name: string, os: Archive['os'], label: string, kind: 'zip' | 'tar.gz'): Archive => {
  const file = `nvs-${version}-${name}.${kind}`
  return { name, os, label, file, url: download(file) }
}

/**
 * One archive for each target `release.yml`'s `publish` requires, in its build order. The optional
 * targets may be missing from a release, so no page links them.
 */
export const archives: Archive[] = [
  archive('linux-x86_64', 'Linux', 'Linux, x86-64', 'tar.gz'),
  archive('linux-aarch64', 'Linux', 'Linux, ARM64', 'tar.gz'),
  archive('windows-x86_64', 'Windows', 'Windows, x86-64', 'zip'),
  archive('macos-aarch64', 'macOS', 'macOS, Apple silicon', 'tar.gz'),
]

/** The archive of one target, by its build matrix name. */
export function archiveFor(name: string): Archive {
  const found = archives.find((a) => a.name === name)
  if (!found) throw new Error(`release.ts: no archive for ${name}`)
  return found
}

/** The checksums of every file in the release. */
export const sums = { file: 'SHA256SUMS', url: download('SHA256SUMS') }

/** The VS Code extension the release attaches. */
export const vsix = { file: `nvs-${version}.vsix`, url: download(`nvs-${version}.vsix`) }
