// Growing a list one element at a time, then walking it - the result set of every query loop.

function run(rows: number): number {
    const items: number[] = [];
    let seed = 1;
    for (let i = 0; i < rows; i++) {
        seed = (seed * 31 + i) % 9973;
        items.push(seed);
    }

    return items.reduce((carry, value) => carry + value, 0) + items.length;
}

console.log(run(1000000));
