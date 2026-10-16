There is no Redis, NATS, SQS or AMQP backend, and no pluggable driver interface. A driver interface
would make transactional enqueue a property of *one* driver rather than of the API
(`rule:concurrency/enqueue-commits-with-your-write`), which is the guarantee the whole design exists
to hold, and it would multiply the surface to specify and test across backends nobody has asked for.

The throughput this buys is a database's throughput — thousands of jobs per second, bounded by write
contention on one table. That ceiling is real and it is documented rather than discovered.

The seam that stays open is the storage layer's internal boundary and not a public interface. Someone
genuinely bounded by database write throughput has outgrown what the queue promises, and a
broker-backed queue would have to say plainly which guarantee it drops.
