`Core\Queue::push` on the queue's own connection enlists in whatever transaction that connection
already has open. If the transaction rolls back the job was never enqueued; if it commits the job is
durable. There is no window in which the order exists and the job that was to send its receipt does
not, and there is no outbox to write.

That is the property a job being a row buys, and it is the whole reason the storage is a table in a
database `Core\Db` already talks to (`rule:core-classes/queue-storage-is-a-table`). Every queue built
on an external broker has an enqueue and a business write in two systems that cannot commit together,
and every team on one rediscovers the outbox pattern — which is to say they rediscover that the
database was the right queue.

Outside a transaction, `push` is its own committed statement, which is the ordinary case and needs
nothing said about it. Pointing `[queue] connection` at a separate database is permitted and silently
gives up this property; so does pushing on a different connection than the queue's
(`rule:concurrency/foreign-connection-enqueue-is-counted`).
