// String-keyed insert and read back - the cache, the index, the lookup table.

function run(entries: number): number {
    const index = new Map<string, number>();
    for (let i = 0; i < entries; i++) {
        index.set("key-" + i, i);
    }

    let total = 0;
    for (let i = 0; i < entries; i++) {
        const found = index.get("key-" + ((i * 3) % entries));
        if (found !== undefined) {
            total = total + found;
        }
    }
    return total;
}

console.log(run(200000));
