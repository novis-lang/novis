// `includes` over an array - the membership test userland reaches for before it reaches for a Set.

function run(size: number, probes: number): number {
    const allowed: string[] = [];
    for (let i = 0; i < size; i++) {
        allowed.push("role-" + i);
    }

    let hits = 0;
    for (let probe = 0; probe < probes; probe++) {
        if (allowed.includes("role-" + ((probe * 7) % (size * 2)))) {
            hits = hits + 1;
        }
    }
    return hits;
}

console.log(run(500, 20000));
