- **An `echo` with several arguments prints the ones before a throw, so a caught call inside one
  leaves half a line in the blessed output.** `echo $n, ": ", Core\Decimal::pow(1.05, 15), "\n";`
  inside a `try` printed `15: ` and then the `catch` printed its own line, which `--bless` froze as
  the example's expected output. Put a call that can throw in its own statement above the `echo`
  whenever the example catches it. [until: reviewed 2026-09-22]
