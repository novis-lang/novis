`Core\Db\Schema` is a value with three interchangeable spellings and no privileged one: **built**
with typed builders, **serialized** to and from a plain array, and **introspected** off a live
connection (`rule:core-classes/schema-introspection`).

The array form is **canonical**. Two schema values are the same schema exactly when their array forms
agree, and `toArray(fromArray(a)) == a` holds over every construct in the vocabulary. It is stated
over the array rather than over the objects because `==` on two objects is identity, there is no
equality hook to override, and a schema has no natural total order. The canonical form is therefore
**ordered**: declaration order for columns, since a `CREATE TABLE` must reproduce it, and name order
for everything else, since nothing observable depends on it — an introspector returning the server's
catalog order would fail the round trip on a database that is not wrong in any way.

The diff runs over that canonical form after normalization, and **never over SQL text**.
Normalization is where this class of tool lives or dies: a unique constraint's implicit index,
integer display widths, `varchar` promotion, column order, identifier case-folding, and the server's
own spelling of a default are each normalized out on both sides. The acceptance criterion is one
property — apply a schema, introspect it back, and the resulting plan is **empty**, on all five
backends. Every normalization rule exists because that property failed without it.
