# Novis for VS Code

This extension adds support for the Novis language to VS Code. It works with `.nvs` files and with
`nvs.toml` projects.

## What you get

- Syntax colours as soon as you open a file.
- Errors and warnings while you type.
- Completion, hover information, go to definition and find references.
- Formatting of the whole file.
- Commands to run the current file and to run its tests.
- A debugger for `.nvs` programs.
- In the Explorer, a letter beside a file that declares one class (`C`), interface (`I`), enum (`E`)
  or type alias (`T`).
- In the Explorer, an `R` beside a file with routes. Hold the pointer over the file to see each route.

## Getting started

The extension needs the `nvs` program. If `nvs` is on your `PATH`, the extension uses it. If not, run
**Novis: Download nvs** from the Command Palette. The `nvs.path` setting has the same **Download nvs**
link. You can also set the path to `nvs` in that setting.

The extension starts when you open a `.nvs` file, or a folder that contains an `nvs.toml` file.

## Colours for class names and methods

Your colour theme decides every colour in a `.nvs` file. Some themes give class names and methods
almost the same colour. You can change this in two ways:

- Pick the **Novis Dark** or **Novis Light** colour theme. Run **Novis: Use Novis Colour Theme**, or
  choose the theme with **Preferences: Color Theme**.
- Keep your theme and turn on `nvs.highlighting.distinctNames`. Novis then adds colour rules for your
  theme to `editor.semanticTokenColorCustomizations` in your user settings. When you turn it off, Novis
  removes these rules.

Open the Settings editor and search for `nvs.highlighting` to see both options.

## Where hidden secrets are still visible

The editor hides the value of a `secret` string while you read the code. The setting is
`nvs.secrets.redact`. This only changes what the editor draws. The value is still in the file, and you
can see it in these places:

- Search results and the Quick Open preview. They show the matching lines outside the editor.
- Diff and source control views. The old value of a secret you changed is visible there.
- The minimap. It draws the text of the file.
- Other extensions. Their hovers, code lenses and web views read the text of the file.
- The clipboard. If you copy a hidden value, the clipboard gets the real value.
- The file on disk, the working tree and the Git history. A secret that is written in the code is
  still in the code.

Keep real secrets out of the source code. Read them from the environment or a secret store instead.

## More information

The documentation is at [novis-lang.org](https://novis-lang.org). You can report a problem on
[GitHub](https://github.com/novis-lang/novis/issues).
