// Templated text: the default way userland renders a line -- a backtick template string where PHP uses sprintf.

function run(rounds: number): number {
    let total = 0;
    for (let i = 0; i < rounds; i++) {
        const line = `user #${i} scored 42`;
        total = total + line.length;
    }
    return total;
}

console.log(run(300000));
