---
summary: a priority queue that answers its smallest element first — `SplPriorityQueue`, `SplMinHeap` and `SplMaxHeap` in one class
keywords: SplPriorityQueue, SplMinHeap, SplMaxHeap, SplHeap, priority queue, min-heap, max-heap, comparator, Comparable, compareTo, scheduling
---

`Core\Heap<T>` keeps its elements ordered so that `peek` and `pop` always answer the **smallest** one. The
order is decided once, at `new`: a comparator passed as the constructor's argument —
`new Core\Heap<T>(fn (T $a, T $b): int => …)`, answering negative, zero or positive as `Core\Arr::sort`'s does
— wins; without one, an object orders by its own `Comparable::compareTo`, and a scalar by its natural
order. A max-heap is therefore the min-heap with its comparator reversed, not another class. `peek` and
`pop` throw on an empty heap — ask `isEmpty` first — and a `foreach` over a heap yields its elements in
`pop` order without removing any of them.

```nvs
<?nvs
class Job implements Comparable {
    public function constructor(public string $name, public int $cost) {}

    public function compareTo(Job $other): int {
        return $this->cost - $other->cost;
    }
}

var $jobs = new Core\Heap<Job>();
$jobs->push(new Job("deploy", 5));
$jobs->push(new Job("lint", 1));
$jobs->push(new Job("build", 3));

var $first = $jobs->peek();
echo "next=", $first->name, " of ", $jobs->count(), "\n";
while (!$jobs->isEmpty()) {
    var $job = $jobs->pop();
    echo $job->name, "\n";
}

var $largest = new Core\Heap<int>(fn (int $a, int $b): int => $b - $a);
$largest->push(3);
$largest->push(9);
$largest->push(5);
foreach ($largest as int $n) {
    echo $n, " ";
}
echo "still ", $largest->count(), "\n";
```
```output
next=lint of 3
lint
build
deploy
9 5 3 still 3
```
