Adds a job that a worker runs later, so a request can answer quickly and leave slow work for the background.

A job is a Novis file and the data it receives. `push` saves it as a row in the database that `[queue] connection` names, and returns an id for the job. A worker takes the row later and runs the file. When you call `push` inside a transaction on that database, the job is saved only if the transaction commits. So an order and the email about that order are saved together, or not at all.

**In plain words:** a to-do note that you put in the same drawer as the order. If the order is thrown away, the note goes with it.

**Good to know:** a job can run more than once, for example after a worker stops in the middle. Write jobs that are safe to run twice.

**The examples below** add one job, stop a duplicate with a key, and save a job together with an order.
