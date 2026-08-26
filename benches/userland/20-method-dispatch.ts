// Calling an override through a base-typed handle - the interface dispatch a framework runs on.

class Shape {
    protected size: number;

    constructor(size: number) {
        this.size = size;
    }

    area(): number {
        return this.size * this.size;
    }
}

class Circle extends Shape {
    override area(): number {
        return this.size * this.size * 3;
    }
}

function run(rounds: number): number {
    const shapes: Shape[] = [new Shape(3), new Circle(4), new Shape(5), new Circle(6)];
    let total = 0;
    for (let i = 0; i < rounds; i++) {
        for (const shape of shapes) {
            total = (total + shape.area()) % 1000003;
        }
    }
    return total;
}

console.log(run(500000));
