// trim / lower / pad - the tidy-up every form field and imported row goes through.

function run(rounds: number): number {
    const raws: string[] = [];
    for (let k = 0; k < 16; k++) {
        raws.push("   Ada  LOVELACE " + "x".repeat(k) + "   ");
    }

    let total = 1;
    for (let i = 0; i < rounds; i++) {
        const tidy = raws[total % 16].trim().toLowerCase();
        const padded = tidy.padEnd(32, ".");
        total = total + padded.length;
        if (padded.includes("lovelace")) {
            total = total + 1;
        }
    }
    return total;
}

console.log(run(300000));
