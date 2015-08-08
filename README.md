This Rust module provides a memory allocation Arena for types that
implement Copy.

[Documentation](https://PeterReid.github.io/copy_arena)

# Examples

```rust
extern crate copy_arena;

use copy_arena::Arena;

let mut arena = Arena::new();
let mut allocator = arena.allocator();

let a: &mut i32 = allocator.alloc(5);
let b: &mut f64 = allocator.alloc_default();
let c: &mut [u8] = allocator.alloc_slice(b"some text");
let b: &mut [usize] = allocator.alloc_slice_fn(10, |idx| idx + 1);
let e: &mut [u32] = allocator.alloc_slice_default(10);
```

# Compared to std::arena::Arena

This differs from the (unstable) Arena in Rust's standard library in
a couple of ways:

 - This Arena only supports `Copy`-able objects -- no destructors are 
   executed.
 - This Arena does not use dynamic borrow checking, saving two RefCells
   and an Rc before getting to the underlying data to allocate from but
   leading to a slightly less convenient API.


