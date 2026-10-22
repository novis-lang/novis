- **A fix to how a task's memory ceiling is armed can turn a `Core\Task` attack red, and the attack is
  the thing that was wrong.** `Core\Task::map` over 20,000 elements with no limit passed while the
  parent ran under its last child's arming. Once the arming traveled with its stack, the 1 MiB stack
  charge per child met the parent's 256 MB ceiling as its rule says. The only report was `floor-debt`,
  a goal later. After a change to `crates/nvs-runtime/src/budget.rs` or the scheduler's switch, run
  `bun nv proofs --run --group 'Core\Task'` in that stage. `until: gone tests/hostile/core/Task/map/01-bad-limits-huge-arrays-and-errors-from-every-task.nvs:ends-early 7 FATAL`
