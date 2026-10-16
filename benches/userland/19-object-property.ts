// Constructing objects and moving values through their properties - the DTO, the entity, the model.
// JavaScript's `%` truncates toward zero as PHP's and Novis's do, so the total going negative on the
// first iteration needs no helper here -- only the Python twin does.

class Point {
    constructor(
        public x: number,
        public y: number,
    ) {}
}

function run(rounds: number): number {
    let total = 0;
    for (let i = 0; i < rounds; i++) {
        const point = new Point(i, i * 2);
        point.x = point.x + point.y;
        point.y = point.y - 1;
        total = (total + point.x + point.y) % 1000003;
    }
    return total;
}

console.log(run(1000000));
