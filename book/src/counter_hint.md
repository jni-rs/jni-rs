# Counter Hint
One solution might do the following:

1. Allocate the Rust Counter on the heap using a
[`Box`](https://doc.rust-lang.org/std/boxed/index.html).
2. Store `*mut Counter` in Java `long` fields.
3. Use `Box::into_raw` and `Box::from_raw` to convert between `*mut Counter` and
   `Box<Counter>` or `&Counter`.
4. Use an additional method called `close()` to drop the Counter.
