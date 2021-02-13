# Counter Discussion

1. Ferris has a `Box<dyn CounterTrait>` that they would like to use from Java.
Can they pass it to Java, like this? Why or why not? If not, does the error
surface at compile time, or runtime?

```rust,noplaypen
{{#include ../projects/completed/jnibookrs/src/counter.rs:ferris_counter}}
```

<details>

`Box<dyn CounterTrait>` is a wide pointer, so it won't fit into a `jlong`. To
resolve this, it must be double-boxed: `Box<Box<dyn CounterTrait>>`. See the [Rust reference on DSTs.](https://doc.rust-lang.org/reference/dynamically-sized-types.html)

</details>

2. This implementation of `get` is bad. Why?
   

```rust,noplaypen
{{#include ../projects/completed/jnibookrs/src/counter.rs:discussion_2_2}}
```

<details>

Using `Box` means that the Rust code is taking ownership of the counter. This
could lead to double frees or use-after-free. For example, let's say that
someone introduces new code that panics between `from_raw` and `into_raw`, and
introduces a panic handler. In this case, the following occurs during `get`:

1. Rust makes a `Box<Counter>`, then panics panics.
2. `panic!` leads to the `Counter` being dropped.
3. The program recovers from the panic.

At this point, the Counter has already been unintentionally freed, and a
use-after-free or double free is likely the next time any function on the
Counter is called.

</details>
