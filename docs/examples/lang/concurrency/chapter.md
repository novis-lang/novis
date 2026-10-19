---
order: 80
title: Tasks, channels and isolates
summary: structured concurrency with `Core\Task`, bounded channels, and `spawn script` isolates that share nothing
keywords: Core\Task, all, map, limit, deadline, TimeoutError, sleep, Core\Task\Channel, send, close, spawn script, await, Core\Script\Handle, isolate, ScriptResult, script.spawn, args, output, capture, inherit, async, await, fibers, fork, parallel, concurrent requests, threads, workers, shared state
---

# No colouring: I/O just waits
# `Core\Task::all`: a fixed set of tasks
# `Core\Task::map`: one task per element
# `limit` and `deadline`
# A child's throw
# `Core\Task\Channel<T>`: a bounded queue between tasks
# Isolates: `spawn script` and `await`
# A child shares nothing
# A child's failure is a value
# What does not exist
