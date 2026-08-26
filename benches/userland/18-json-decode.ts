// Parsing then reading the fields - MWL validates into a declared type where JSON.parse hands back `any`.

interface Order {
    id: number;
    name: string;
    status: string;
    amount: number;
}

function run(rounds: number): number {
    const payloads: string[] = [];
    for (let k = 0; k < 16; k++) {
        payloads.push(
            '{"id":' + (4711 + k) + ',"name":"order-4711","status":"open","amount":1999}',
        );
    }

    let total = 1;
    for (let i = 0; i < rounds; i++) {
        const order = JSON.parse(payloads[total % 16]) as Order;
        total = (total + order.id + order.amount + order.name.length) % 1000003;
    }
    return total;
}

console.log(run(200000));
