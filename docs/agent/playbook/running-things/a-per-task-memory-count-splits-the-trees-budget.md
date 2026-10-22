- **A memory count kept per task stack splits a request tree's budget, and every interleaving test
  still passes.** Two requests on one core read their own bytes correctly, but a `Core\Task::map`
  child's 1 MiB stack charge and the results it files leave the parent's reading. Each child then
  reads the tree's ceiling alone, and `Core\Task/map/01` breaches at step 4 with 430 MB held. Charge a
  turn to the running *tree*, and run `bun nv proofs --run --group 'Core\Task'` before trusting a
  change to how the balance crosses a switch. See `crates/nvs-runtime/src/budget.rs`.
  `until: gone crates/nvs-runtime/src/budget.rs:Per *thread*, read as per *request*`
