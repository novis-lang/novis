Decodes a source once and runs each pipeline on it. Returns one result per pipeline, in the same order.

`Codec::variants` needs a source and a list of plans. The source and each plan have the same shape as
for `Codec::run`. Each plan runs on its own copy of the decoded image, so one plan does not change the
image that the next plan sees. Each plan has its own `output`, so one call can return a file for one
plan and only a size for another.

When one plan cannot run, `Codec::variants` throws an error and returns no results. An empty list of
plans returns an empty array.

**Good to know:** `variants` decodes the source only once, and it is one call to the image component.
To write several sizes of one upload, it is faster than several calls to `Codec::run`.
`Image::variants` builds the plans for you.
