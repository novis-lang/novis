// Serialising a nested payload - what every API response ends with.

function run(rounds: number): number {
    const tags = ["billing", "eu", "priority"];
    let total = 0;
    for (let i = 0; i < rounds; i++) {
        const payload = {
            id: i,
            name: "order-" + i,
            status: "open",
            amount: 1999,
            tags: tags,
        };
        total = total + JSON.stringify(payload).length;
    }
    return total;
}

console.log(run(200000));
