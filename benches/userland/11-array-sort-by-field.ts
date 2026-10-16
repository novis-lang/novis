// Ordering records by one field - the leaderboard, the sorted table, the comparator every app has.

class Row {
    constructor(
        public name: string,
        public score: number,
    ) {}
}

function run(size: number, rounds: number): string {
    const rows: Row[] = [];
    // BigInt for the LCG only, for the reason 10-array-sort's comment gives.
    let seed = 12345n;
    for (let i = 0; i < size; i++) {
        seed = (seed * 1103515245n + 12345n) % 2147483648n;
        rows.push(new Row("row-" + i, Number(seed % 100000n)));
    }

    let total = 1;
    let top = "";
    for (let round = 0; round < rounds; round++) {
        rows[round % size] = new Row("edit-" + round, total % 100000);
        const ordered = [...rows].sort((a, b) => a.score - b.score);
        const first = ordered[0];
        if (first !== undefined) {
            total = total + first.score + 1;
            top = first.name;
        }
    }
    return top + "/" + total;
}

console.log(run(50000, 20));
