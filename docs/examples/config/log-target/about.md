Where the engine writes a failure it had to report itself.

When a request goes wrong Novis climbs a short ladder: the program's own last words, then the script
the deployment named, and at the bottom a floor written into the engine, which records the failure
with no script involved. This key names where that last record goes — the standard error stream, a
file, or the system log — and with nothing written it is standard error.

A service is different. When the service manager starts Novis and this key is not set, the records
go to `logs/nvs.log` in the data folder. A file target is limited in size by `[log] max_size` and
`[log] keep`.

Moving it is the operator's alone. A program able to choose where the report of its own failure is
written could choose somewhere nobody reads, which is the same as never being reported at all.

The spelling is checked when the configuration file is read: the one moment the engine cannot afford
to complain about its own configuration is the moment it is already reporting a failure.

The example prints the destination on duty and shows a program being turned away from all three
spellings of it.
