// Split a text and tally it - the single most-written userland loop there is.

function run(rounds: number): number {
    const text =
        "the quick brown fox jumps over the lazy dog while the dog sleeps and the fox runs";
    const words = text.split(" ");
    const extras = "alpha beta gamma delta epsilon zeta eta theta".split(" ");

    let total = 1;
    for (let round = 0; round < rounds; round++) {
        const counts = new Map<string, number>();
        for (const word of words) {
            counts.set(word, (counts.get(word) ?? 0) + 1);
        }
        counts.set(extras[total % 8], 1);
        total = (total + counts.size + Math.max(...counts.values())) % 1000003;
    }
    return total;
}

console.log(run(100000));
