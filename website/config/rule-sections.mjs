/**
 * How a rulebook chapter is cut into pages.
 *
 * The repository owns the rules and their order: `data/rules/<topic>.json` names a
 * chapter and lists its rules in the order they are meant to be read,
 * `data/rules/<topic>/<slug>.json` is one rule's record, and
 * `docs/rules/<topic>/<slug>.md` is its prose. None of that
 * carries a section heading, because a chapter in the repository is one document a
 * person scrolls. A chapter on the web is not: Security alone is 89 rules and some
 * 14,000 words, which is a page nobody finishes.
 *
 * So the cut lives here, and only here. A section is a **contiguous run** of a
 * chapter's rules, named by its first rule's slug — never by an index, which every
 * inserted rule would shift. `bun nv render --website` walks each chapter in
 * the repository's order and starts a new section wherever `from` matches, so:
 *
 *   - a rule added to the middle of a chapter joins the section it was written into,
 *   - a rule added to the end joins the last section,
 *   - a `from` naming a rule that no longer exists fails the sync, loudly, rather than
 *     silently merging two sections into one.
 *
 * A chapter with exactly one section has no section pages: its rules render on the
 * chapter page itself, so a short chapter costs one click rather than two.
 *
 * Aim for 6 to 13 rules a section — around 1,000 to 2,500 words, a page that reads in
 * one sitting. The count is a target and not a check: where the material genuinely runs
 * long (containment, the scheduler) a longer section beats an arbitrary cut.
 */

/**
 * The sentence a chapter page opens with, above its section cards.
 *
 * The repository has no such sentence and should not grow one: a chapter there is read
 * from the top, and its first rule is its introduction. A chapter here is a landing
 * page somebody arrives at from a search result, and it has to say what it is about
 * before it lists 89 links.
 *
 * A chapter cut into one section leads with that section's blurb instead, so nothing
 * here would be read — those three are absent on purpose.
 */
export const chapterLeads = {
  programs:
    'Who the language is for, how a name reaches the file that declares it, and what ships in the box.',
  types:
    'Every binding declares its type and keeps it. This is the closed grammar those types are written in — and the one place a value converts without being asked.',
  expressions:
    'What counts as true, what counts as equal, and the two constructs PHP has no spelling for.',
  statements:
    'How a name resolves, where a program is allowed to keep state, and which PHP spellings do not parse at all.',
  classes:
    'Everything a program declares lives in a class body — and nothing about a class is decided by a method happening to have a particular name.',
  errors:
    'A throw is a checked return, not an unwind. What that costs, what happens when nothing catches it, and what a developer-facing message is made of.',
  tooling:
    'The commands around the compiler: the terminal, the formatter, doc comments, the PHP converter, and two telemetry opt-ins that are off until you say otherwise.',
  ide: 'One resilient parse, one language server, and editor clients thin enough that neither of them holds any language logic — plus everything security means for a file that is open in an editor rather than serving a request.',
  testing:
    'Testing is part of the language rather than a package you choose — and a feature is not finished until its feature proofs exist.',
  security:
    "The reason Novis exists. Isolation, capabilities, and two compile-time qualifiers that keep untrusted data and secrets out of the places where they do damage. Every rule here holds while the language is running; what an editor conceals on a screen is the editor's chapter, not this one.",
  concurrency:
    'Structured tasks, durable jobs and persistent connections, over one scheduler that never moves a running task between cores.',
  'core-api': 'What may live in `Core`, and the twenty shape rules every member of it obeys.',
  'core-classes':
    "The standard library's own contracts, class by class — running a program, escaping HTML, talking to a database, decoding an image.",
  observability:
    'Metrics, traces and exit hooks, read off instrumentation the runtime already carries rather than probes added for the purpose.',
  'http-server':
    'What the server does before your code runs, and what it still guarantees when your code goes wrong.',
  routing:
    'A route is compiled into a table, matched once before any application code runs, and dispatched by nothing.',
  config:
    'One TOML file, an ordered tree of includes over it, and three classes deciding what a running request is allowed to change.',
  packaging:
    'How compiled code is cached, how a program becomes a single file, and what a package or an extension is allowed to do once it is there.',
  'php-migration':
    "Every departure from PHP's observable behaviour, listed — and what the converter does with the code you already have.",
}

/** @typedef {{ slug: string, title: string, blurb: string, from: string }} RuleSection */

/** @type {Record<string, RuleSection[]>} */
export const ruleSections = {
  programs: [
    {
      slug: 'claims-and-priorities',
      title: 'What Novis claims',
      blurb: 'Who the language is for, the three things it promises, and the ordering that decides every trade below.',
      from: 'audience',
    },
    {
      slug: 'names-and-files',
      title: 'Names, files and programs',
      blurb: 'How a name reaches the file that declares it — while compiling, with no loader running anywhere.',
      from: 'implementing',
    },
    {
      slug: 'the-framework',
      title: 'The first-party framework',
      blurb: 'Novis ships the framework, split into a privileged half in the binary and an opinionated half as a package.',
      from: 'first-party-framework',
    },
  ],

  types: [
    {
      slug: 'declarations-and-numbers',
      title: 'Declarations and numbers',
      blurb: 'Every binding declares its type and keeps it. The numeric tower, its one implicit conversion, and what overflow does.',
      from: 'declaration',
    },
    {
      slug: 'text-and-literal-types',
      title: 'Text, bytes and literal types',
      blurb: 'A string is UTF-8 for its whole lifetime, bytes is its unencoded peer, and a literal can be a type of its own.',
      from: 'ordering',
    },
    {
      slug: 'arrays-and-property-keys',
      title: 'Arrays and property keys',
      blurb: "PHP's ordered hash with a declared element type, and the type whose values are a class's own property names.",
      from: 'arrays',
    },
    {
      slug: 'unions-and-conversion',
      title: 'Unions, narrowing and conversion',
      blurb: 'What a union permits, the four spellings of narrowing, and the single conversion operator that replaced the cast.',
      from: 'unions-and-mixed',
    },
    {
      slug: 'closures',
      title: 'Closures and callables',
      blurb: 'One closure literal, one callable type, capture by value with no `use` clause, and the variance that follows.',
      from: 'callable-is-a-closure',
    },
    {
      slug: 'objects-and-shapes',
      title: 'Objects, shapes and class references',
      blurb: 'The opaque top of every class type, anonymous object literals, structural shapes, and a class as a value.',
      from: 'object-top',
    },
  ],

  expressions: [
    {
      slug: 'truthiness-and-equality',
      title: 'Truthiness and equality',
      blurb: 'Six positions test truthiness and every other bool stays checked; one equality operator, with one row per type.',
      from: 'truthy-positions',
    },
    {
      slug: 'conversion-and-intrinsics',
      title: 'Conversion and the intrinsics',
      blurb: 'The failable conversion that answers null instead of throwing, and what the compiler prepares from a literal argument.',
      from: 'intrinsic-literals',
    },
    {
      slug: 'pipeline-and-catch',
      title: 'The pipeline and the catch expression',
      blurb: 'Two constructs PHP has no spelling for: a parse-time substitution, and a typed catch that guards one expression.',
      from: 'pipeline-substitution',
    },
  ],

  statements: [
    {
      slug: 'names-and-require',
      title: 'Names, namespaces and `require`',
      blurb: 'One function decides what every name means, nothing gets a second name, and `require` is the only inclusion construct.',
      from: 'nvs-is-the-only-open-tag',
    },
    {
      slug: 'where-state-lives',
      title: 'Where state lives',
      blurb: 'A program holds state in five declared places. There is no `global`, no function static, and no host-populated variable.',
      from: 'static-is-a-member-modifier',
    },
    {
      slug: 'by-reference-and-exit',
      title: 'By-reference and termination',
      blurb: 'A by-reference binding is written `inout` and said again at the call site; `&` does not parse and `die` is refused.',
      from: 'inout-is-the-by-reference-spelling',
    },
  ],

  classes: [
    {
      slug: 'declaring-a-class',
      title: 'Declaring a class',
      blurb: 'Everything lives in a class body. Constructors, promotion, and the proof that every property is assigned before it is read.',
      from: 'no-free-functions-or-constants',
    },
    {
      slug: 'properties',
      title: 'Properties, hooks and observers',
      blurb: '`lateinit` for what the constructor cannot fill, hooks for what a read or write should run, and an interface where PHP had `__get`.',
      from: 'lateinit',
    },
    {
      slug: 'interfaces-and-delegation',
      title: 'Interfaces and delegation',
      blurb: 'There is no `trait`: shared behaviour is a default method, and shared state is a field you delegate to.',
      from: 'member-conflict-is-an-error',
    },
    {
      slug: 'no-magic',
      title: 'Behaviour is declared, never magic',
      blurb: 'No method changes a class by being present. Ordering, string conversion and dumping each go through a named interface.',
      from: 'comparable',
    },
    {
      slug: 'copying-and-serializing',
      title: 'Copying and serializing',
      blurb: 'Two copy depths, neither of which a class customizes, and a serialization format closed to everything it did not write.',
      from: 'two-copy-depths',
    },
  ],

  enums: [
    {
      slug: 'enums',
      title: 'Enums',
      blurb: 'An enum declares a new, closed, named integer type — and costs nothing beyond the integer it is.',
      from: 'declaration',
    },
  ],

  iteration: [
    {
      slug: 'iteration',
      title: 'Iteration',
      blurb: 'What `for` and `foreach` drive, the two interfaces that carry every iteration, and what a generator is and is not.',
      from: 'for-init-clause',
    },
  ],

  attributes: [
    {
      slug: 'attributes',
      title: 'Attributes',
      blurb: 'An attribute is a shape literal on a declaration — inert data from the moment it is parsed, matched by shape rather than by name.',
      from: 'inert-metadata',
    },
  ],

  errors: [
    {
      slug: 'how-an-error-travels',
      title: 'How an error travels',
      blurb: 'A throw is a checked return, not an unwind. What that buys, what it costs, and how deep recursion is allowed to go.',
      from: 'throwable-hierarchy',
    },
    {
      slug: 'the-escalation-ladder',
      title: 'The escalation ladder',
      blurb: 'Four tiers between a failing request and a dead server, none of them retried, ending at a native floor that gives up.',
      from: 'on-limit',
    },
    {
      slug: 'diagnostics-and-logging',
      title: 'Diagnostics and logging',
      blurb: 'Every developer-facing output is one closed record. Five producers build it, and the sink in force picks the rendering.',
      from: 'diagnostic-record',
    },
    {
      slug: 'ambiguous-input',
      title: 'Ambiguous input is refused',
      blurb: 'Where two readings of the same bytes are possible, Novis refuses the whole message rather than repairing it.',
      from: 'ambiguous-input-refused',
    },
  ],

  tooling: [
    {
      slug: 'what-the-toolchain-is',
      title: 'What the toolchain is',
      blurb: 'No REPL, reflection and parsing built in, and the two claims the project will not make against Python.',
      from: 'shebang-opens-code-mode',
    },
    {
      slug: 'the-terminal',
      title: 'The terminal and command-line programs',
      blurb: 'Every `echo` has a sink, every control byte is neutralized, styling is a value, and a command table is built while compiling.',
      from: 'echo-always-has-a-sink',
    },
    {
      slug: 'the-formatter',
      title: 'The formatter',
      blurb: 'One canonical style, no configuration, no reflowing of what you wrote — and a fixed point over the input bytes.',
      from: 'fmt-is-one-canonical-style',
    },
    {
      slug: 'doc-comments-and-metadata',
      title: 'Doc comments and `nvs meta`',
      blurb: 'A doc comment is exactly `///`, and one JSON document is the machine-readable source every renderer consumes.',
      from: 'doc-comment-is-three-slashes',
    },
    {
      slug: 'nvs-convert',
      title: 'Converting PHP',
      blurb: 'One rule table read through two modes, every branch carrying a tier, and not one input byte dropped on the floor.',
      from: 'convert-php-front-end',
    },
    {
      slug: 'benchmarks-and-telemetry',
      title: 'Benchmarks, telemetry and the update check',
      blurb: 'Two independent opt-ins, both off by default, a closed set of counters, and a process serving traffic that never uploads.',
      from: 'bench-engine-list-is-data',
    },
  ],

  ide: [
    {
      slug: 'the-resilient-parse',
      title: 'The resilient parse',
      blurb: 'One grammar, one tree. The parser always returns something, a node it invented says so, and the file is reproducible byte for byte.',
      from: 'one-grammar-one-tree',
    },
    {
      slug: 'the-language-server',
      title: 'The language server',
      blurb: 'Synchronous, one analysis thread, a closed request set, and diagnostics that stop at the first phase that failed.',
      from: 'one-crate-and-one-extension-grow-in-place',
    },
    {
      slug: 'code-actions',
      title: 'Code actions and quick fixes',
      blurb: 'An action may only write text the compiler already determined — never a choice, never an alias, never a guess.',
      from: 'a-code-action-ships-only-a-fix-a-diagnostic-already-knows',
    },
    {
      slug: 'highlighting-and-completion',
      title: 'Highlighting, completion and references',
      blurb: 'Two highlighting layers that ship names and no colours, and completion that offers only what the compiler already derived.',
      from: 'highlighting-is-two-layers',
    },
    {
      slug: 'testing-the-editor',
      title: 'Testing the editor',
      blurb: 'An LSP answer is frozen as a case file with a cursor, a request and its exact rendering — and coverage is inferred, not declared.',
      from: 'case-files-have-their-own-grammar',
    },
    {
      slug: 'one-server-thin-clients',
      title: 'One server, thin clients',
      blurb: 'Language smarts and formatting have one implementation each. An editor client holds none of either.',
      from: 'one-server-two-thin-clients',
    },
    {
      slug: 'the-vs-code-extension',
      title: 'Inside the VS Code extension',
      blurb: 'What the extension contributes, why it builds no UI the editor already has, and where its clients live in the tree.',
      from: 'tasks-carry-a-problem-matcher',
    },
    {
      slug: 'phpstorm-and-the-debugger',
      title: 'PhpStorm and the debugger',
      blurb: 'The same server behind another editor, and a debug adapter that is complete before any debugger UI exists.',
      from: 'phpstorm-bridges-to-the-same-server',
    },
    {
      slug: 'security-in-the-editor',
      title: 'Security in the editor',
      blurb: 'What security means once a file is open rather than running: the process on the wire, the binary the client will trust, and a secret concealed on screen. None of it is a language guarantee.',
      from: 'stdout-belongs-to-the-protocol',
    },
  ],

  testing: [
    {
      slug: 'writing-a-test',
      title: 'Writing a test',
      blurb: 'A test is a method with an attribute, running in its own isolate, asserting on two values of one type.',
      from: 'test-attribute',
    },
    {
      slug: 'doubles-and-the-runner',
      title: 'Doubles, determinism and the runner',
      blurb: 'Structural doubles, a fixed clock and seed, property tests, inline snapshots — and a runner that refuses to call nothing green.',
      from: 'doubles',
    },
    {
      slug: 'feature-proofs',
      title: 'Feature proofs',
      blurb: 'A feature is finished when it has a description, a test from both sides, three examples, a measured figure and an attack against it.',
      from: 'nvst-is-separate',
    },
    {
      slug: 'coverage-and-probes',
      title: 'Coverage, tracing and the debug probes',
      blurb: 'One per-request flag word at fixed probe sites, exporting formats existing tooling already reads — never a second compiled tier.',
      from: 'capability-closure-test',
    },
    {
      slug: 'measuring-performance',
      title: 'Measuring performance',
      blurb: 'Counted semantic work rather than wall-clock, a per-PR guard separate from the historical dashboard, and an append-only history.',
      from: 'bench-counters',
    },
    {
      slug: 'continuous-integration',
      title: 'Continuous integration',
      blurb: 'One workflow, three lanes, and one table that decides what a given diff has to run.',
      from: 'ci-lanes',
    },
  ],

  security: [
    {
      slug: 'isolates',
      title: 'Isolates',
      blurb: 'Running another script shares nothing but compiled code. Values cross by copy, failure crosses as data, and the budget is the tree.',
      from: 'isolate-shares-nothing',
    },
    {
      slug: 'closed-doors',
      title: 'The closed doors',
      blurb: 'No FFI, no `eval`, no scheme-dispatching path, no cross-request state. Four doors closed by construction, reopened by nothing.',
      from: 'script-spawn-capability',
    },
    {
      slug: 'capabilities',
      title: 'Capabilities and authority',
      blurb: 'The check lives inside the function that performs the effect, and authority belongs to the namespace enclosing the code.',
      from: 'capability-check-at-the-door',
    },
    {
      slug: 'scopes-and-denial',
      title: 'Scopes, denial and the address policy',
      blurb: 'How a path scope is decided, why a denial is catchable, and why the network address policy lives inside the capability itself.',
      from: 'path-scope-canonicalise-then-prefix',
    },
    {
      slug: 'tainted-data',
      title: 'Tainted data and its sinks',
      blurb: 'Everything from outside arrives `tainted`, it poisons what it touches, and a sink is any parameter a parser will execute.',
      from: 'tainted-qualifier',
    },
    {
      slug: 'laundering',
      title: 'Laundering',
      blurb: 'The only ways out of `tainted`: a member named for the one sink it is safe for, a typed capture, or a written assertion.',
      from: 'launderers-are-sink-named',
    },
    {
      slug: 'secrets',
      title: 'Secrets',
      blurb: 'A second, independent qualifier. Every sink refuses it, it crosses no boundary, comparing two is constant-time, and no rendering the toolchain prints carries one in the clear.',
      from: 'secret-qualifier',
    },
    {
      slug: 'extensions-and-qualifiers',
      title: 'Qualifiers across an extension',
      blurb: 'An extension manifest may only tighten. Contagion is the default, and no spelling anywhere removes a qualifier.',
      from: 'extension-manifest-only-tightens',
    },
    {
      slug: 'protocols-and-tokens',
      title: 'Protocols, tokens and CSRF',
      blurb: 'One TLS client, a closed protocol roster, an algorithm that comes from the key, and CSRF on by default.',
      from: 'one-tls-client',
    },
    {
      slug: 'bidi-and-passwords',
      title: 'Bidirectional text and password hashes',
      blurb: 'A two-counter predicate against direction-flipping source, and a hash surface that writes one algorithm and reads exactly two.',
      from: 'bidi-predicate',
    },
  ],

  concurrency: [
    {
      slug: 'tasks',
      title: 'Tasks',
      blurb: 'Every task is a child of the one that started it. Control never leaves a group with a child still running.',
      from: 'one-scheduler',
    },
    {
      slug: 'deferred-and-cross-request-state',
      title: 'Deferred work and cross-request state',
      blurb: 'Work that outlives the response but not the request, and the only way a value outlives the request that made it.',
      from: 'after-response-outlives-the-connection',
    },
    {
      slug: 'connections',
      title: 'Persistent connections',
      blurb: 'A WebSocket or an SSE stream is a root isolate with a loop in it, upgraded from a request that then ends.',
      from: 'a-connection-is-a-root-isolate',
    },
    {
      slug: 'the-job-queue',
      title: 'The job queue',
      blurb: 'Enqueue commits with your own write. Four members, at-least-once delivery, finite attempts, and a dead letter that is kept.',
      from: 'enqueue-commits-with-your-write',
    },
    {
      slug: 'running-a-job',
      title: 'Running a job',
      blurb: 'A job is a root isolate on a recorded budget. Which process runs it is configuration, and there is no pluggable driver.',
      from: 'cancel-is-a-race-it-can-lose',
    },
    {
      slug: 'the-scheduler',
      title: 'The scheduler underneath',
      blurb: 'Futures, wakers and parking: how a task suspends, what a wake is permission to do, and why a task never migrates cores.',
      from: 'one-future-per-connection',
    },
  ],

  'core-api': [
    {
      slug: 'what-belongs-in-core',
      title: 'What belongs in Core',
      blurb: 'Core means always present. Six ordered tests place a candidate into one of five outcomes, and the roster records every one.',
      from: 'core-means-always-present',
    },
    {
      slug: 'naming-and-shape',
      title: 'Naming and shape',
      blurb: 'Twenty shape rules every member obeys: the subject first, a verb that predicts the return type, and casing that is a hard error.',
      from: 'shape-rules',
    },
    {
      slug: 'everything-is-written',
      title: 'Everything is written down',
      blurb: 'Visibility, wire-format participation and qualifier behaviour are all declared. Nothing about a member is inferred structurally.',
      from: 'written-visibility',
    },
    {
      slug: 'one-way-to-do-each-thing',
      title: 'One way to do each thing',
      blurb: 'Nothing mutates, nothing reads ambient state, no mutable twin, no mode string, no encoding argument, one range convention.',
      from: 'nothing-mutates',
    },
    {
      slug: 'parameters-and-options',
      title: 'Parameters and the options bag',
      blurb: 'Optional knobs are one trailing shape literal — and required, optional and nullable stay three separate questions.',
      from: 'callback-receives-value-and-key',
    },
    {
      slug: 'shape-parameters',
      title: 'Shape parameters',
      blurb: 'One checked type with disjoint arms, flattened away at the ABI so no shape exists at run time.',
      from: 'field-wise-precedence',
    },
    {
      slug: 'lifetimes-and-absences',
      title: 'Lifetimes, signatures and what is absent',
      blurb: 'Anything with a lifetime is an object, a signature is taken over a payload, and every removed PHP built-in is accounted for by name.',
      from: 'a-lifetime-is-an-object',
    },
  ],

  'core-classes': [
    {
      slug: 'processes-and-files',
      title: 'Processes and files',
      blurb: 'Running another program is argv only — there is no shell string anywhere — and a temporary directory sweeps itself.',
      from: 'cli-arguments',
    },
    {
      slug: 'regex-html-and-introspection',
      title: 'Regex, HTML and introspection',
      blurb: 'A two-tier regex engine, auto-escaping output with one typed bypass, read-only reflection, and an inert parse tree.',
      from: 'regex-two-tiers',
    },
    {
      slug: 'connecting-to-a-database',
      title: 'Connecting to a database',
      blurb: 'One database API, every statement prepared, three deny-by-default capabilities, and defaults that close what PHP leaves open.',
      from: 'db-one-api',
    },
    {
      slug: 'running-a-statement',
      title: 'Running a statement',
      blurb: 'One placeholder is one value. Columns have natural types, transactions are closures, and one error kind spans every driver.',
      from: 'db-parameters',
    },
    {
      slug: 'schemas',
      title: 'Schemas and convergence',
      blurb: 'A schema is a value, a plan is its difference from a live database, and something the schema omits is reported, never dropped.',
      from: 'schema-is-a-value',
    },
    {
      slug: 'codecs-sessions-and-signatures',
      title: 'Codecs, sessions, rate limits and signatures',
      blurb: 'One attribute generates a codec from the declared property list; sessions start explicitly; both rate-limit verbs are their own job.',
      from: 'derive-attribute',
    },
    {
      slug: 'uris-and-images',
      title: 'URIs and images',
      blurb: 'An image is an immutable value carrying a plan — nothing decodes until a terminal runs it, under a cap read off the header.',
      from: 'uri-removable-components',
    },
    {
      slug: 'pdf-and-spreadsheets',
      title: 'PDF and spreadsheets',
      blurb: 'Two first-party extensions that perform no I/O and write nothing a reader would execute, byte-identical for equal input.',
      from: 'pdf-render-has-no-io',
    },
  ],

  observability: [
    {
      slug: 'metrics',
      title: 'Metrics',
      blurb: 'Three verbs for three kinds, nine series that exist the moment an exporter is configured, and a name fixed to one kind for good.',
      from: 'the-runtime-exports-what-it-already-measures',
    },
    {
      slug: 'traces',
      title: 'Traces and spans',
      blurb: 'Exactly four things become a span, an inbound `traceparent` is continued, and sampling is decided once at the root.',
      from: 'four-kinds-become-a-span',
    },
    {
      slug: 'exit-hooks',
      title: 'Exit hooks',
      blurb: 'A request-local hook that observes how the request ended and never steers it — and the two endings that fire no hook at all.',
      from: 'script-on-exit',
    },
  ],

  'http-server': [
    {
      slug: 'listening-and-admission',
      title: 'Listening and admission',
      blurb: 'What the server is and is not, how mounts expand at boot, and the valve that answers 503 before an isolate exists.',
      from: 'two-deployments-and-nothing-a-proxy-owns',
    },
    {
      slug: 'resolving-a-request',
      title: 'Resolving a request',
      blurb: 'Five steps from bytes to entry point, forwarded headers that are read only from a trusted peer, and a trailing slash that means something.',
      from: 'a-request-resolves-in-five-steps',
    },
    {
      slug: 'methods-bodies-and-static-files',
      title: 'Methods, bodies and static files',
      blurb: 'HEAD runs as GET, a preflight is answered before any code runs, and a route that never reads a body allocates nothing.',
      from: 'head-runs-as-get',
    },
    {
      slug: 'headers-cors-and-cookies',
      title: 'Response headers, CORS and cookies',
      blurb: 'What a deployment that configured nothing already sends, and why CORS stays closed until an origin is named.',
      from: 'the-access-log-is-a-mode-default',
    },
    {
      slug: 'sessions',
      title: 'Sessions',
      blurb: 'A session is a record its store issued. Four operations, loaded once, written whole only when it changed, and never locked.',
      from: 'a-session-store-answers-four-operations',
    },
    {
      slug: 'uploads',
      title: 'Uploads',
      blurb: 'Parts arrive lazily through one member, bounded by two separate caps, and there is no temporary file anywhere.',
      from: 'an-upload-is-received-only-through-files',
    },
    {
      slug: 'the-http-client',
      title: 'The HTTP client',
      blurb: 'No spelling for "wait forever", one deadline over the whole call, opt-in jittered retry, and an address pinned before connecting.',
      from: 'no-spelling-for-an-unbounded-wait',
    },
    {
      slug: 'containment',
      title: 'Containment',
      blurb: 'Nothing a request can send terminates a worker. Four bounded tiers, a watchdog that sheds rather than kills, and one stated residue.',
      from: 'a-requests-blast-radius-is-bounded-at-four-tiers',
    },
  ],

  routing: [
    {
      slug: 'declaring-a-route',
      title: 'Declaring a route',
      blurb: 'A route is an attribute compiled into a table. Structural precedence, captures that narrow to closed sets, and no wildcard verb.',
      from: 'routes-are-compiled-not-registered',
    },
    {
      slug: 'matching',
      title: 'Matching, not dispatching',
      blurb: 'The router stops at a name and typed parameters. It carries nothing invocable, and the server dispatches none of it.',
      from: 'matching-is-not-dispatching',
    },
    {
      slug: 'links-and-the-api-document',
      title: 'Links and the API document',
      blurb: 'Link building is checked against the compiled table, and the OpenAPI document is generated from that same table while compiling.',
      from: 'link-name-and-params-are-checked',
    },
  ],

  config: [
    {
      slug: 'the-file-and-the-tree',
      title: 'The file and the tree',
      blurb: 'It is TOML, it is a deployment file rather than a project manifest, and a host with no file at all is fully configured.',
      from: 'the-file-is-nvs-toml-and-it-is-toml',
    },
    {
      slug: 'includes-and-ownership',
      title: 'Includes, ownership and secrets',
      blurb: 'One ordered stream where later wins and every override is recorded — over files nobody but the runtime account may write.',
      from: 'include-takes-a-path-or-a-dir',
    },
    {
      slug: 'changeability-classes',
      title: 'Changeability classes',
      blurb: 'The file states defaults, not ceilings. Three classes decide what a running request may change, and a refusal is never a clamp.',
      from: 'one-parser-for-the-boot-path-and-config-set',
    },
    {
      slug: 'reloading-and-control',
      title: 'Reloading and control',
      blurb: 'An edit reaches the next request without a restart, and the running process is controlled over one local socket and no network surface.',
      from: 'an-edit-reaches-the-next-request-without-a-restart',
    },
    {
      slug: 'run-modes',
      title: 'Run modes',
      blurb: 'Exactly two, defaulting to production, selected by the file or a flag and never by an environment variable.',
      from: 'two-modes-and-the-default-is-production',
    },
    {
      slug: 'application-blocks',
      title: 'Application blocks',
      blurb: 'An application is its entry file path. A mount routes; an `[[app]]` block sets policy, bounded by the global ceiling.',
      from: 'an-application-is-its-entry-file-path',
    },
    {
      slug: 'scheduled-work',
      title: 'Scheduled work',
      blurb: 'A config block, not a runtime API: five cron fields, a mandatory scope, a lease across a fleet, and a missed fire that stays missed.',
      from: 'scheduled-work-is-a-config-block',
    },
    {
      slug: 'stores-and-caches',
      title: 'Stores, sockets and the compiled-unit cache',
      blurb: 'Where a Unix socket may be written, why the extension set is in every cache key, and which opcache directives an operator alone owns.',
      from: 'cache-shared-is-the-grant-over-the-configured-store',
    },
  ],

  packaging: [
    {
      slug: 'the-artifact-cache',
      title: 'The compiled-unit cache',
      blurb: 'One immutable file addressed by its content and environment, verified whole before a page becomes executable.',
      from: 'an-artifact-is-one-immutable-content-addressed-file',
    },
    {
      slug: 'single-file-builds',
      title: 'Single-file builds',
      blurb: 'A bundle is source appended to a copy of the host binary, found by a footer, carrying a notice generated from the dependency graph.',
      from: 'nvs-build-compile-appends-the-program-to-a-copy-of-the-host',
    },
    {
      slug: 'packages',
      title: 'Packages',
      blurb: 'A package is its digest, reached by an ordinary autoload line, and nothing in one runs before your program does.',
      from: 'a-package-name-is-vendor-slash-name',
    },
    {
      slug: 'the-version-contract',
      title: 'The registry and the version contract',
      blurb: 'An append-only transparency log, retraction rather than deletion, and a ladder a dependency break goes down before it is forwarded.',
      from: 'the-registry-keeps-an-append-only-log',
    },
    {
      slug: 'extensions',
      title: 'Extensions',
      blurb: 'A third-party extension is a sandboxed WebAssembly component: fresh per request, no ambient authority, statically typed at the call.',
      from: 'an-extension-is-a-sandboxed-wasm-component',
    },
    {
      slug: 'running-as-a-service',
      title: 'Running as a service',
      blurb: 'An operator installs a service from the command line. The installer fails closed, and nothing about it is on the request path.',
      from: 'pdf-decoding-ships-in-the-image-package',
    },
  ],

  'php-migration': [
    {
      slug: 'divergences',
      title: 'Where Novis diverges',
      blurb: 'Every departure from PHP is listed, and each exists because PHP left a binding untyped. Absent storage is never a zero value.',
      from: 'every-divergence-is-deliberate-and-listed',
    },
    {
      slug: 'converting-and-completing',
      title: 'Converting and completing PHP',
      blurb: 'What the converter does with a trait or a shell call, and how every PHP built-in name stays a completion candidate.',
      from: 'a-deprecation-is-a-refusal',
    },
  ],
}
