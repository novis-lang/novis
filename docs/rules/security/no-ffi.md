There is no `FFI`-equivalent, no `com_dotnet`-equivalent, and no mechanism by which userland code
causes native code to be loaded into the process.

Loading native code destroys memory safety, because one bad write corrupts arbitrary memory, and it
destroys request isolation, because one segfault takes down every in-flight request in a single
process. A userland FFI reaches both outcomes from a worse starting point than an extension does: the
unsafe pointer arithmetic is now written by an application developer under deadline rather than by
someone who chose to write an extension.

The wasm component tier exists so this is unnecessary. Wrapping an existing C or Rust library is its
stated purpose, and it delivers the same capability with a memory boundary, a capability grant, a CPU
deadline and a memory cap — which is the whole of what an FFI gives up.
