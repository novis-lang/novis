---
claim: Testing is a language feature, not a framework you install
category: tooling
comparedTo: [PHP, Node.js]
proof: 'ADR 0079: `#[Test]` attributes, the assertion surface, fixtures, property tests and benchmarks are part of `Core\Test` and the compiler — `nvs test` works in every project with zero dependencies, and each test runs in its own isolate.'
tradeoff: 'One blessed way means less choice: if you prefer a different test framework''s style, there is no marketplace of alternatives.'
draft: true
weight: 10
---

PHPUnit, Jest, Vitest, Mocha — every project starts by choosing, installing and
configuring a test framework, and every framework reinvents isolation badly. In Novis
the test runner ships in the same binary as the compiler, assertions are type-checked
Core members (`assertEquals<T>` on mismatched types is a compile error), and test
isolation is the same request isolation the server uses.
