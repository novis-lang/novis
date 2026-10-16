Hovering a value whose type carries `tainted` or `secret`, or an argument where such a value reaches a
sink, shows the path the qualifier took inside the method: where it entered, each step that carried
it, and the call that laundered it, each with its line. The checker records that path only under a
switch that `nvs-lsp` alone sets, so a compile for running, `nvs check` and `nvs build` record nothing.

A path starts where the qualifier entered the method: a source (`rule:security/tainted-sources`), a
conversion out of `mixed` (`rule:security/taint-propagation`), or a parameter, property or return type
declared with the qualifier. Each assignment, concatenation and interpolation that carried it is a
step. A launderer (`rule:security/launderers-are-sink-named`) is the step where the qualifier ended,
and the hover marks it.

The path stops at the method's boundary. A qualifier crosses a boundary only through a declared type,
so a parameter declared `tainted` is where the value entered this method, and the hover names that
declaration. That declaration is everything the method knows about its caller.

- **Memory:** under the switch, one list of steps per qualified value, bounded by the statements of its
  method and freed with the analysis in the editor process. Without the switch, one predicted branch per
  recording site at compile time, and nothing at run time.

**Declined:**
- **Recording the path on every compile.** Nothing but the editor reads it.
- **A path across methods.** It would be a whole-program flow analysis, and the declared qualifier on
  a parameter already states what crosses.
- **A panel or a request of its own for the path.** A hover is a surface every editor has
  (`rule:ide/the-extension-builds-no-ui-the-editor-already-has`).
