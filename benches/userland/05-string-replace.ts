// Substitution over a template - the placeholder fill every mailer and view helper does.

function run(rounds: number): number {
    const template = "Hello {name}, your order {order} ships to {city} on {date}.";
    const names: string[] = [];
    for (let k = 0; k < 16; k++) {
        names.push("ab".repeat(k + 1));
    }

    let total = 1;
    for (let i = 0; i < rounds; i++) {
        let line = template.replaceAll("{name}", names[total % 16]);
        line = line.replaceAll("{order}", "A-1000");
        line = line.replaceAll("{city}", "Vienna");
        line = line.replaceAll("{date}", "2026-08-25");
        total = total + line.length;
    }
    return total;
}

console.log(run(200000));
