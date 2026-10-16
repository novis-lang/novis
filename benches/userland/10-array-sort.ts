// Sorting a list of numbers, from the same seeded input on both sides.

function run(size: number, rounds: number): number {
    const source: number[] = [];
    // The LCG runs in BigInt: `seed * 1103515245` reaches ~2.4e18, past the 2^53 a float64 holds
    // exactly, and PHP and Novis compute it in 64-bit integers. One of the two arithmetic exceptions
    // the README names; everything below it is ordinary `number`.
    let seed = 12345n;
    for (let i = 0; i < size; i++) {
        seed = (seed * 1103515245n + 12345n) % 2147483648n;
        source.push(Number(seed % 100000n));
    }

    let total = 1;
    for (let round = 0; round < rounds; round++) {
        source[round % size] = total % 100000;
        const ordered = [...source].sort((a, b) => a - b);
        total = total + ordered[0] + ordered[ordered.length - 1];
    }
    return total;
}

console.log(run(50000, 20));
