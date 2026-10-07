Counts how many bits differ between two image hashes.

You get a hash from `Image::hash`. Two images that look the same give hashes with a small distance,
even when one is smaller, larger or saved in another format. Two different images give a large
distance. A distance of 0 means the hashes are equal.

Both hashes must be of the same `HashKind`. Each kind has its own length, and two hashes of
different lengths throw a `LogicError`.

**Good to know:** the largest distance is the number of bits in the hash: 64 for `Perceptual`, 128
for `Difference` and 256 for `Average`. Choose your limit for "the same image" from that number.

**The examples below** show the distance of two short hashes, a check for a duplicate upload, and
the error for two kinds.
