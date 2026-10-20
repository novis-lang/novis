Every way a subclass or an implementor can declare a member again over its parent's is a conformance
case, generated from one table. **Each cell compiles and runs the declaration its rule names, reading a
value of its declared type, or is the compile error its rule names at the line it names.** No cell is
skipped, and none is a gap record.

The table is the module doc of `bun nv soundness` (`tools/nv/cmd/soundness.ts`), and each row cites the
rule that gives its verdict. A cell is one member kind, one type relation between the two declarations
and one path a program reaches the member by:

- **members:** a method's parameter and return, an interface method's parameter and return, a static
  method's parameter and return, `compareTo`'s parameter, an instance property (plain, promoted,
  `lateinit`, hooked, `array<T>`), a `readonly` property, a static property, and a class or interface
  constant read through `self::`, the class name and `static::`;
- **relations:** the same type, a subclass, a superclass, an unrelated class, `int` under `int|string`,
  `int` over `float`, `float` over `int`, `T` under `?T`, `?T` over `T`, a union member under the union,
  and a wider union;
- **paths:** a variable of the parent's type, of the child's type, of an interface's type, `static::`,
  an erased `object` or `mixed` access, a `property<T>` key, and a first-class callable.

`--write` writes the cells to `tests/conformance/soundness/`, one file per member kind for the cells that
run and one for the cells that are compile errors. `bun nv verify` runs `--check`, which fails when a
committed file differs from what `--write` writes, and the `.nvst` step runs the cells like any case. A
cell's expectation is the rule's verdict and is never changed to match the binary: where the rule is
wrong, the rule and the table change together. `--probe` runs each cell alone, to name the one that
fails when a whole file does.

A rule-driven suite pins what a rule says, and no case exists where no rule spoke. The matrix asks the
question for every form instead, and the checked build
(`rule:testing/probes-on-is-a-tested-configuration`) asks it for every typed read of every case. The
cost is CI time: the cells run with the rest of the suite.

**Declined:**
- **A skip list of failing cells.** It held the first findings while they were fixed, and a list that
  outlives its fixes is a place a regression can hide. A failing cell is a red verify.
- **Hand-written cases per relation.** The relations multiply with the member kinds and the paths, and a
  hand-written set covers the combinations someone thought of.
