use std::mem;
use std::cmp;
use std::usize;
use std::default::Default;
use std::slice;
use std::fmt;

struct Chunk {
    data: Vec<u8>,
    next: Option<Box<Chunk>>,
}

impl Chunk {
    fn attempt_alloc(&mut self, size: usize, align: usize) -> Option<*mut u8> {
        let start = round_up(self.data.len(), align);

        if size <= self.data.capacity() && start <= self.data.capacity() - size {
            Some(unsafe {
                self.data.set_len(start + size);
                self.data.as_mut_ptr().offset(start as isize)
            })
        } else {
            None
        }
    }
}

pub struct Arena {
    head: Chunk,
}

impl Arena {
    pub fn new() -> Arena {
        Arena::with_capacity(1000)
    }

    pub fn with_capacity(capacity: usize) -> Arena {
        Arena {
            head: Chunk{
                data: Vec::with_capacity(capacity),
                next: None,
            }
        }
    }

    fn add_chunk(&mut self, chunk_size: usize) {
        let mut new_head = Chunk {
            data: Vec::with_capacity(chunk_size),
            next: None,
        };

        mem::swap(&mut self.head, &mut new_head);
        self.head.next = Some(Box::new(new_head));
    }

    pub fn allocator(&mut self) -> Allocator {
        Allocator {
            arena: self
        }
    }

    pub fn capacity(&self) -> usize {
        let mut iter: &Chunk = &self.head;
        let mut total_capacity = 0;
        loop {
            total_capacity += iter.data.capacity();
            match iter.next {
                None => { return total_capacity; }
                Some(ref next) => { iter = next; }
            }
        }
    }
}

impl fmt::Debug for Arena {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_fmt(format_args!("Arena {{ capacity_bytes: {} }}", self.capacity()))
    }
}

#[derive(Debug)]
pub struct Allocator<'a> {
    arena: &'a mut Arena,
}

#[inline]
fn round_up(base: usize, align: usize) -> usize {
    (base + (align - 1)) & !(align - 1)
}

impl<'a> Allocator<'a> {
    fn alloc_raw(&mut self, size: usize, align: usize) -> &'a mut u8 {
        loop {
            match self.arena.head.attempt_alloc(size, align) {
                Some(x) => { return unsafe { mem::transmute(x) } },
                None => {
                    // Double the current allocation (or the asked for one), but don't overflow.
                    let minimum_reasonable = cmp::max(self.arena.head.data.len(), size);
                    let new_chunk_size = 2 * cmp::min(minimum_reasonable, usize::MAX/2);
                    self.arena.add_chunk(new_chunk_size);
                }
            }
        }
    }

    pub fn alloc<T: Copy>(&mut self, elem: T) -> &'a mut T {
        let memory = self.alloc_raw(mem::size_of::<T>(), mem::min_align_of::<T>());
        let res: &'a mut T = unsafe { mem::transmute(memory) };
        *res = elem;
        res
    }

    pub fn alloc_default<T: Copy+Default>(&mut self) -> &'a mut T {
        self.alloc(Default::default())
    }

    fn alloc_slice_raw<T>(&mut self, len: usize) -> &'a mut [T] {
        let element_size = cmp::max(mem::size_of::<T>(), mem::min_align_of::<T>());
        assert_eq!(mem::size_of::<[T;7]>(), 7 * element_size);
        let byte_count = element_size.checked_mul(len).expect("Arena slice size overflow");
        let memory = self.alloc_raw(byte_count, mem::min_align_of::<T>());
        let res: &'a mut [T] = unsafe { slice::from_raw_parts_mut( mem::transmute(memory), len) };
        res
    }

    pub fn alloc_slice<T: Copy>(&mut self, elems: &[T]) -> &'a mut [T] {
        let mut slice = self.alloc_slice_raw(elems.len());
        for (dest, src) in slice.iter_mut().zip(elems.iter()) {
            *dest = *src;
        }
        slice
    }

    pub fn alloc_slice_fn<T: Copy, F>(&mut self, len: usize, mut f: F)-> &'a mut [T]
        where F: FnMut(usize) -> T
    {
        let mut slice = self.alloc_slice_raw(len);
        for (idx, dest) in slice.iter_mut().enumerate() {
            *dest = f(idx)
        }
        slice
    }

    pub fn alloc_slice_default<T: Copy+Default>(&mut self, len: usize)-> &'a mut [T] {
        self.alloc_slice_fn(len, |_| Default::default())
    }
}


#[test]
fn construct_simple() {
    let mut arena = Arena::with_capacity(4);
    let mut allocator = arena.allocator();

    let x: &mut i32 = allocator.alloc(44);
    let y: &mut u8 = allocator.alloc(3);
    let z: &mut u32 = allocator.alloc(0x11223344);
    let w: &mut f64 = allocator.alloc_default();
    assert_eq!(*x, 44);
    assert_eq!(*y, 3);
    assert_eq!(*z, 0x11223344);
    assert_eq!(*w, 0.0);
}

#[test]
fn construct_slices() {
    let mut arena = Arena::with_capacity(4);
    let mut allocator = arena.allocator();

    let s = ::std::str::from_utf8(allocator.alloc_slice(b"abc")).unwrap();
    let xs: &[i32] = allocator.alloc_slice_fn(10, |idx| (idx as i32)*7);
    let ys: &[u64] = allocator.alloc_slice_default(4);

    assert_eq!(xs[9], 9*7);
    assert_eq!(s, "abc");
    assert_eq!(ys[0], 0);
}
