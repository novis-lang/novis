Coverage is inferred, never declared. `nvs lsp-test --coverage` prints the matrix of request × syntactic
construct, taking the construct from the node the cursor actually resolved to. A case cannot claim coverage
it does not have, and nobody maintains a list by hand.

The guard test `every_request_answers_every_construct` reads that matrix and fails naming each empty cell
— a gate that enumerates its source of truth rather than counting. Rust integration tests are kept beside
`.lspt` for what they are genuinely better at, the resilient parser's own invariants over a corpus, and
never as the only mechanism, because that would make coverage invisible to the gate the loop stops on.
