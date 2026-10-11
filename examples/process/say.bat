@echo off
rem `examples/process.nvs` names this file so its third line is evidence
rem about the target's KIND rather than about its absence. `Core\Process` refuses
rem a batch target on every platform, so nothing here is ever executed --
rem `Core\Process::run` declines before the operating system is asked.
echo unreachable
