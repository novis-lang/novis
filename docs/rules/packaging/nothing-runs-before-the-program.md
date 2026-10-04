**No install scripts, no post-install hooks, no build step, no code generation, no macros.** There is
no point in the fetch, resolve, verify or compile pipeline at which a package's code executes. The
first time a line of a dependency runs is when your program calls it.

Most of this was already true — there is no `eval` (`rule:security/no-eval`), attributes are inert
anonymous objects, the only compiler-recognised attributes are a closed `Core`-owned list, and there is
no macro expander for a package to hook. This rule adds the one thing that was missing, *no build
lifecycle, ever*, and thereby closes the category: the entire post-install attack class has nowhere
to execute.

A package that needs generated code generates it into its own repository **before** publishing,
where a human reviews the output and the digest covers it. The generated file is source like any
other.

A package containing a top-level statement with an observable effect produces no effect from
`nvs fetch`, `nvs build` or `nvs vendor` — only from a call. There is no legitimate exception: in a
language with no FFI and no native compilation step available to userland, an allowance for
"packages that genuinely need it" would exist only to be abused, and if such a need ever appears
the rule is re-argued from scratch rather than amended.
