// A match with capture groups - how userland parses a log line, a header or an id.

const PATTERN = /(\w+) \/orders\/(\d+) (\d{3}) (\d+)ms/;

function run(rounds: number): number {
    const lines: string[] = [];
    for (let k = 0; k < 16; k++) {
        lines.push("2026-08-25 12:04:31 " + "A".repeat(k + 1) + " /orders/4711 200 138ms");
    }

    let total = 1;
    for (let i = 0; i < rounds; i++) {
        const found = PATTERN.exec(lines[total % 16]);
        if (found !== null) {
            total = total + found[1].length;
        }
    }
    return total;
}

console.log(run(200000));
