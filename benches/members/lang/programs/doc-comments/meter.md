# Meter

`Meter` adds a run of numbers into one running total.

## The wrap

Each round adds the remainder of a division by `Meter::WRAP`.

## A run

`Meter::run` runs a number of rounds and returns the running total.
