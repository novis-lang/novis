Adds a job that a worker runs later, so a request can answer quickly and leave slow work for the background.

A job is a Novis file and the data it receives. `push` saves it as a row in the database that `[queue] connection` names, and returns an id for the job. A worker takes the row later and runs the file. When you call `push` inside a transaction on that database, the job is saved only if the transaction commits. So an order and the email about that order are saved together, or not at all.

**Good to know:** a job can run more than once, for example after a worker stops in the middle. Write jobs that are safe to run twice.

The job reads its data with `Core\Script::args()`. Every string in that data is `tainted`, which means it came from outside the program. Check it before you use it in SQL or as a file path. A `secret` value cannot be pushed.

**The examples below** add one job, stop a duplicate with a key, and save a job together with an order.
