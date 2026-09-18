Which database this deployment's background jobs are written to.

Novis keeps queued work in an ordinary table rather than in a broker of its own, and this key names
the connection that table lives in — one of the database blocks the same file already describes.
There is no default: a deployment that queues work says where the rows go, and one that writes a
queue block without this key is refused at boot rather than left quietly enqueueing nothing.

**In plain words:** name the application's own database. An enqueue is then a row written in the
same transaction as the work that caused it, so a job for an order that was rolled back never
exists to be worked, and a job that exists is never for work that did not happen.

The key belongs to whoever runs the deployment, and it is read once when the process starts:
a program that could redirect the queue would be sending this deployment's work into a database it
was never granted, and a connection swapped under running workers would strand every job already
claimed against a database nothing will report back to.

The example prints where this deployment stores its jobs and is turned away trying to store them
anywhere else.
