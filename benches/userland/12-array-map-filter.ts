// map / filter / reduce over a list - the pipeline that replaced the for loop, and in JavaScript
// it is spelled with those three methods chained.

function run(size: number, rounds: number): number {
    const source: number[] = [];
    for (let i = 0; i < size; i++) {
        source.push(i);
    }

    let total = 1;
    for (let round = 0; round < rounds; round++) {
        source[round % size] = total % 1000;
        total =
            total +
            source
                .map((n) => n * 2)
                .filter((n) => n % 3 === 0)
                .reduce((carry, n) => carry + n, 0);
    }
    return total;
}

console.log(run(100000, 20));
