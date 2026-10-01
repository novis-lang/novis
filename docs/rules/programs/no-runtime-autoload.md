Nothing in Novis loads code in response to a name being referenced at run time. A reference to a class,
interface, enum or `type` alias is resolved to the file declaring it while checking, through
`rule:programs/autoload`'s map, and the program's file graph is closed before any user code runs.

There is no loader stack to register with, no registration call, no manifest file, no configuration
home, no walk-up root search, no classmap, no PSR-0 underscore rule, and no "load these files
unconditionally" list. A registered loader would be process-global mutable state driving a load of
arbitrary code, it would have to run during name resolution — which has already finished by the time
any user code exists — and it would reopen the closed `require` graph a bundled executable depends on.

There is likewise no allow/deny list over what may be loaded. A check on class loading is not a
security boundary: nothing stops code *referencing* a denied class, so the check only moves the failure
later. A deployment that must exclude a module does not ship its directory.

Classes cannot be loaded from a database, a generated file, or anywhere but the filesystem at compile
time, and a name held in a string can never pull in a new file — `Core\Reflect`'s lookup by name
reaches only the compiled program. A string *literal* under `as class<T>` is a name written in the
source, not one held at run time, and is loaded while compiling (`rule:types/class-reference`).
