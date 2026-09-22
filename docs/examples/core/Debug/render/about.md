Builds the same text `Core\Debug::dump` writes, and returns it instead of writing it.

You get the rendering as a `Core\Cli\Text`. You can print it, put it in a message, keep it in an
array, or compare it with the rendering of another value. It names the type of the value, the length
of every string and the class of every object. A control character such as a tab is made visible, a
very deep or very long structure is cut short, and a property declared `secret` is shown as
`[redacted]`.

The text has no line break at the end, so you decide where the line ends. `echo` of a rendering
prints it as it is, because those bytes are already prepared for the screen and are not changed a
second time.

**The examples below** show a rendering printed, renderings kept for a summary at the end, and two
renderings compared.
