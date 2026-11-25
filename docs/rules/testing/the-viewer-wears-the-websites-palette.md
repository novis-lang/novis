The debug viewer is drawn in the website's palette, in both of the website's themes, with the
system's own fonts, so a developer who opens it from the website or the editor sees one product.

**The palette is the website's block, copied once.** `viewer/src/styles/tokens.css` starts with the
seven colours of the website's palette block in `website/src/styles/custom.css` (`--nv-crimson`,
`--nv-orange`, `--nv-gold`, `--nv-blue`, `--nv-ink`, `--nv-fog`, `--nv-ground`), with the same
values, and derives every other colour from them with `color-mix()` as the website does, in a dark and
a light theme and in the Display-P3 copy. `bun nv viewer --check` fails when the seven values in the
two files differ, so a re-tint of the website reaches the viewer in the same change. The copy is
deliberate: the viewer is bundled into the binary and the website is not, so neither imports the
other.

**The viewer names roles, never places.** A rule outside `tokens.css` reads a role: `--surface`,
`--line`, `--text`, `--muted`, `--link`, `--wash`, `--ok`, `--warn`, `--bad`, and the event tones
below. A view picks no colour of its own.

**A colour has one meaning, as on the website.** Green is ok, blue is interaction and information,
orange is a warning, red is an error, and gold is a highlight, such as a repeated query. A request's
time split and its timeline give each event kind the website's diagram tone: `query` blue, `http`
violet, `cache` and `session` green, `file` gold, `gc` orange, `spawn` crimson, and the time spent in
the program's own code the faint gray. The legend under a split names each tone, because a colour
alone carries nothing for a reader who cannot tell two apart.

**The signature gradient and crimson are rare.** The crimson-to-orange gradient paints the header's
hairline and the logo tile, and never text or a row in a list. Crimson fills one control per page, its
primary action, which on a request is *Replay*. Every text colour keeps WCAG AA contrast on every
surface it is drawn on, in both themes
(`rule:testing/the-viewer-shows-the-stream-and-hands-off-every-artifact`).

**The viewer draws with the system's fonts and ships none.** No font file is embedded, served or
fetched, the stylesheet has no `@font-face`, and the service has no font route. `tokens.css` declares
two stacks once, and no other rule names a face:

- `--sans`: `'Segoe UI Variable Text', system-ui, -apple-system, 'Segoe UI', Roboto, 'Noto Sans',
  sans-serif`, for all text. The first name gives Windows 11 its variable face where it is installed.
- `--mono`: `ui-monospace, 'Cascadia Mono', 'SF Mono', Menlo, Consolas, 'DejaVu Sans Mono',
  'Liberation Mono', monospace`, for code, paths, statements and ids.

What makes a system face look made for the page is set once in `viewer/src/styles/base.css`: a scale
of five sizes (11, 12, 13, 15 and 20 pixels), text at weights 400 and 600 with 700 for the brand
alone, code at 400 and 700, `font-variant-numeric: tabular-nums` on every figure,
`font-variant-ligatures: none` on code so `!=` stays two characters, a letter spacing of `-0.01em`
from 15 pixels up, and `font-optical-sizing: auto`. No weight between these is used, because Consolas
and Menlo have none. Buys: no bytes in the binary, no licence notice, no build step, and text drawn on
the first frame. Costs: the viewer looks a little different on each system, its letters are not the
website's, and on Linux it is as good as the fonts installed there. The palette, the gradient and the
components carry the likeness.

**Icons are inline SVG paths in one module,** `viewer/src/ui/icon.tsx`, drawn on a 16-pixel grid with
one stroke width, coloured by `currentColor`.

**Motion is transform, opacity and colour only,** 140 ms, and none under
`prefers-reduced-motion`. A new row fades in. Nothing on screen moves when a record arrives while a
view is paused.

- **Memory:** none at run time beyond the stylesheet, which is part of the one CSS file the binary
  already embeds.

**Declined:**
- **Starlight's `--sl-*` variable names.** They name the website's framework, which the viewer does
  not run, and a rename there would silently change the viewer's meaning.
- **An icon package.** It is a second runtime dependency, which every binary would ship.
- **A font embedded in the binary or fetched from a network.** The binary grows for a look, the
  notice grows, and a fetched font reaches outside the machine.
- **A colour picked per view.** Two views would give one meaning two colours.
