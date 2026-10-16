Returns how busy the machine is, as three numbers.

The numbers are averages over the last 1, 5 and 15 minutes. Each one counts the processes that are
running or waiting to run. A number larger than `Core\Os::cpuCount()` means some processes are
waiting for a core.

Windows does not keep a load average. There, this method throws a `RuntimeError`. A program that
must run on every system catches that error and decides what to do without the numbers.

The examples show how to read the three numbers, how to handle a system with no load average, and
how to wait before a large job when the machine is busy.
