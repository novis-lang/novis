---
order: 95
title: Testing
summary: `#[Test]` methods, `nvs test`, a directory of test files as one program, fixtures, data rows, generated inputs, the assertion roster, fixed clocks and seeds, the report formats, and benchmarks with counted work
keywords: test, #[Test], nvs test, test directory, test suite, bootstrap, assertEquals, assertSame, assertTrue, assertNull, assertCount, assertThrows, expected exception, data rows, TestWith, Fixture, property, Property, property-based testing, generated inputs, random inputs, shrink, counterexample, setup, skip, retries, flaky, Core\Test, Core\Test\Failure, --format json, junit, fixed clock, seed, .nvst, how to test, unit test, which test, expected output, benchmark, Bench, #[Bench], nvs test --bench, count, allocations
---

# A test is a method marked `#[Test]`
# Every test is its own isolate, and the constructor is `setUp`
# `#[Test(...)]` options
# `#[Core\Test\Fixture]`: built once, injected by type
# `#[Core\Test\TestWith]`: data rows
# `#[Core\Test\Property]`: generated inputs
# Hooks: `#[BeforeClass]`, `#[AfterClass]` and `#[AfterEachTest]`
# Assertions
# Running tests: `nvs test`
# `#[Bench]` and `nvs test --bench`
