---
order: 95
title: Testing
summary: `#[Test]` methods, `nvs test`, a directory of test files as one program, fixtures, data rows, the assertion roster, fixed clocks and seeds, and the report formats
keywords: test, #[Test], nvs test, test directory, test suite, bootstrap, assertEquals, assertSame, assertTrue, assertNull, assertCount, assertThrows, expected exception, data rows, TestWith, Fixture, setup, skip, retries, flaky, Core\Test, Core\Test\Failure, --format json, junit, fixed clock, seed, .nvst, how to test, unit test, which test, expected output
---

# A test is a method marked `#[Test]`
# Every test is its own isolate, and the constructor is `setUp`
# `#[Test(...)]` options
# `#[Core\Test\Fixture]`: built once, injected by type
# `#[Core\Test\TestWith]`: data rows
# Hooks: `#[BeforeClass]`, `#[AfterClass]` and `#[AfterEachTest]`
# Assertions
# Running tests: `nvs test`
