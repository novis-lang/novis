---
claim: Untrusted input cannot reach an injection sink — the compiler refuses it
category: security
comparedTo: [PHP, Node.js]
proof: 'ADR 0024 and ADR 0088: every value arriving from outside the process is `tainted`, every SQL/HTML/shell/header sink refuses a tainted argument, and the only way through is the one named launderer for that sink.'
tradeoff: 'You must handle input deliberately. Code that "just concatenates" a query string does not compile, and porting such code means restructuring it — that friction is the feature, but it is friction.'
draft: true
weight: 10
---

SQL injection and XSS are not bugs you hunt in Novis — they are programs that do not
compile. Everything a request sends (parameters, headers, cookies, uploaded file names)
carries the `tainted` qualifier in the type system. A tainted value flows freely through
ordinary code, but the functions that hand text to an interpreter — SQL, HTML, the
terminal, a `printf` template, a regex pattern — refuse it at compile time. Binding
parameters, escaping HTML and proving path containment are the named ways through, and
each removes the qualifier only for its own sink.

PHP and Node.js both leave this to discipline: every `$pdo->query("… $input")` or
template-string query is one code review away from production.
