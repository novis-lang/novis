A unique key in the schema vocabulary means the standard's unique key on all five backends: rows whose
key columns are all non-null are distinct, and a row with a null in any key column collides with
nothing.

Four backends give that directly. **SQL Server's emitter writes a filtered unique index** for a key
over any nullable column —

```sql
CREATE UNIQUE INDEX <key> ON <table> (<columns>) WHERE <column> IS NOT NULL;
```

— with one `IS NOT NULL` conjunct per nullable key column, and the plain `ADD CONSTRAINT … UNIQUE`
where every key column is `not null`.

**That is a spelling, not a vocabulary growth.** A `Core\Db\Schema` declares a unique key over columns
and nothing else; the `WHERE` is the emitter's, the way SQLite's emitter already writes
`CREATE UNIQUE INDEX` where the others write a constraint, and
`rule:core-classes/schema-vocabulary-is-closed` still keeps a partial index out of what a program may
express. The catalog reader matches the index back to the key that asked for it, predicate and all, so
a second `plan` over a converged database is empty. **Where it cannot, `plan` refuses** on a SQL
Server table holding a nullable unique column, naming the table and the key: a refusal costs an
operator a manual step, while DDL that will not converge re-proposes itself on every deployment. A
build over an existing table is `Locking` and inside a `CREATE TABLE` it is `Safe`
(`rule:core-classes/schema-plan`).

The alternative is refused for a reason that outlives SQL Server: a `not null` column carrying a
generated token per row cannot be added to a table that already holds rows, so it describes a schema
no existing deployment can converge to, while a nullable column arrives as a `Safe` step. Uniform null
semantics are what `rule:core-classes/db-one-api`'s one-API promise means for a program's own schema,
and `rule:core-classes/queue-storage-is-a-table`'s `dedupe_pending` is one reader of them rather than
the reason for them.
