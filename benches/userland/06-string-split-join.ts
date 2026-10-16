// split/join round trips - how userland parses a CSV line, a path or a header.

function run(rounds: number): number {
    const lines: string[] = [];
    for (let k = 0; k < 16; k++) {
        lines.push("id" + k + ",name,email,city,country,created,updated,status,score,notes");
    }

    let total = 1;
    for (let i = 0; i < rounds; i++) {
        const fields = lines[total % 16].split(",");
        const back = fields.join("|");
        total = total + fields.length + back.length;
    }
    return total;
}

console.log(run(200000));
