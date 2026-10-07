// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.
//
// Portions ported from upstream, under this notice:
//
// Copyright 2016 The Fuchsia Authors
// Copyright (c) 2015 Google, Inc. All rights reserved
// Copyright (c) 2008-2015 Travis Geiselbrecht
//
// Permission is hereby granted, free of charge, to any person obtaining
// a copy of this software and associated documentation files
// (the "Software"), to deal in the Software without restriction,
// including without limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of the Software,
// and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// The above copyright notice and this permission notice shall be
// included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
// MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE
// SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

//! A heap tuned for space, as one instance per memory domain.
//!
//! Ported from Fuchsia's `zircon/kernel/lib/heap/cmpctmalloc/cmpctmalloc.cc`
//! at https://github.com/misttech/fuchsia revision
//! `0703a5c2d237c2c45afddbf4deb6e1cfc2b571ac`, with its tests as the unit tests below.
//! Upstream is one kernel heap that grows by pages; here each domain has its
//! own heap, which changes it so:
//!
//! - **Instances, no growth:** `CmpctHeap` is one heap over regions its caller
//!   gives it, split into chunks of at most `MAX_CHUNK`. Upstream's page
//!   allocator, growth, return of memory to the OS, and its cache of a free
//!   allocation are gone, so every call takes bounded time and the heap hands
//!   out nothing outside its regions.
//! - **Header:** upstream's profiling cookie, and the 32-bit size that made
//!   room for it, become one `usize` size: two words, which keeps allocations
//!   8-byte aligned on 32-bit targets.
//! - **Added:** `reset` for a domain restart; `owns`, a constant-time check
//!   that a pointer starts a live block; `resize_in_place` for `realloc`; and
//!   counters of the peak in use and of failures.
//! - **`memalign`:** checks for overflow, pads small requests so the block can
//!   become a free area on 64 bit, and gives back the tail past the aligned
//!   block, which upstream left in it.
//! - **Removed:** kernel ASAN, tracing, counters, `cmpct_dump`, the cookie API,
//!   and the debug fills. Upstream's cookie, cached-allocation, and grow-retry
//!   tests have nothing left to test.
//! - **Headers by reference:** `HeaderRef` and `FreeAreaRef` wrap the header
//!   pointers upstream passes around, so that walking and relinking areas is
//!   safe code once a pointer has been checked at the boundary.
//!
//! Freelist entries are kept in linked lists with 8 different sizes per binary
//! order of magnitude, and the header size is two words, with eager coalescing on
//! free.
//!
//! ## Concepts
//!
//! Region:
//!   A contiguous range of memory given to the heap with `add_region`. Upstream
//!   called these OS allocations and asked the kernel for them; here the caller
//!   supplies them and they stay for the life of the heap. Initial layout:
//!
//!   Low addr =>
//!     Header left_sentinel -- Marked as allocated, `left` pointer null.
//!     FreeArea memory_area -- Marked as free, with appropriate size,
//!                             and pointed to by a free bucket.
//!     [bulk of usable memory]
//!     Header right_sentinel -- Marked as allocated, size zero
//!   <= High addr
//!
//!   A region larger than `MAX_CHUNK` is split into chunks of at most that size,
//!   each with its own sentinels, so that each free area maps to a bucket.
//!
//! Memory area:
//!   A sub-range of a region. Used to satisfy `alloc` and `memalign` calls. Can
//!   be free and live in a free bucket, or be allocated and managed by the user.
//!
//!   Memory areas, both free and allocated, always begin with a `Header`,
//!   followed by the area's usable memory. `Header::size` includes the size of
//!   the header. `HeaderRef::left` points to the preceding area's header.
//!
//!   The low bit of `Header::left` is set when the area is free and lives in a
//!   free bucket; check it with `HeaderRef::is_free`. A free area's header is
//!   followed by the doubly-linked free list pointers of `FreeArea`, which chain
//!   the area off of the appropriately-sized free bucket.
//!
//! Free buckets:
//!   Freelist entries are kept in linked lists with 8 different sizes per binary
//!   order of magnitude: `free_lists[NUMBER_OF_BUCKETS]`.
//!
//!   Allocations are always rounded up to the nearest bucket size. This would
//!   appear to waste memory, but in fact it avoids some fragmentation.
//!
//!   Consider two buckets with size 512 and 576 (512 + 64). Perhaps the program
//!   often allocates 528 byte objects for some reason. When we need to allocate
//!   528 bytes, we round that up to 576 bytes. When it is freed, it goes in the
//!   576 byte bucket, where it is available for the next of the common 528 byte
//!   allocations.
//!
//!   If we did not round up allocations, then (assuming no coalescing is
//!   possible) we would have to place the freed 528 bytes in the 512 byte
//!   bucket, since only memory areas greater than or equal to 576 bytes can go
//!   in the 576 byte bucket. The next time we need to allocate a 528 byte object
//!   we do not look in the 512 byte bucket, because we want to be sure the first
//!   memory area we look at is big enough, to avoid searching a long chain of
//!   just-too-small memory areas on the free list. We would not find the 528
//!   byte space and would have to carve out a new 528 byte area from a large
//!   free memory area, making fragmentation worse.
//!
//! `free` behavior:
//!   Freed memory areas are eagerly coalesced with free left and right
//!   neighbors.
//!
//! Every operation does a fixed amount of work: a scan of at most
//! `BUCKET_WORDS` bitmap words, then constant-time unlinking, splitting, and
//! coalescing. Nothing walks a free list.

use core::mem::size_of;
use core::ptr::{self, NonNull};

/// The largest bucket covers sizes below `2^HEAP_ALLOC_VIRTUAL_BITS`. A free
/// area, including its header, must round to at most that.
const HEAP_ALLOC_VIRTUAL_BITS: u32 = 21;

/// Buckets for allocations. The smallest 15 buckets are 8, 16, 24, etc. up to
/// 120 bytes. After that we round up to the nearest size that can be written
/// /^0*1...0*$/, giving 8 buckets per order of binary magnitude. The freelist
/// entries in a given bucket have at least the given size, plus the header
/// size. On 64 bit, the 8 byte bucket is useless, since the freelist header is
/// 16 bytes larger than the header, but we have it for simplicity.
const NUMBER_OF_BUCKETS: usize = 1 + 15 + (HEAP_ALLOC_VIRTUAL_BITS as usize - 7) * 8;

/// 32-bit words in the bitmap of non-empty buckets.
const BUCKET_WORDS: usize = (NUMBER_OF_BUCKETS + 31) >> 5;

/// If a header's `left` field has this bit set, it is free and lives in a free
/// bucket.
const FREE_BIT: usize = 1;

/// Allocations are aligned to this, the alignment of `max_align_t` on the
/// 32-bit targets rivet builds for.
pub(crate) const HEAP_ALIGNMENT: usize = 8;

/// The most regions a heap tracks, for `owns` and `reset`.
pub(crate) const MAX_REGIONS: usize = 4;

/// All individual memory areas on the heap start with this.
///
/// Upstream also held a 32-bit `cookie` for the kernel's heap profiler, and a
/// 32-bit `size` to make room for it. Without it the header is two words, so
/// on 32-bit targets an allocation is aligned to 8, as `max_align_t` requires.
#[repr(C)]
struct Header {
    /// Pointer to the previous area in memory order. The lower bit is used to
    /// store extra state: see `FREE_BIT`. The left sentinel will have null in
    /// the address portion of this field. Left and right sentinels will always
    /// be marked as "allocated" to avoid coalescing.
    left: *mut Header,
    /// The size of the memory area in bytes, including this header. The right
    /// sentinel will have 0 in this field.
    size: usize,
}

zr::static_assert!(size_of::<Header>() == 2 * size_of::<usize>());
zr::static_assert!(size_of::<Header>() & (HEAP_ALIGNMENT - 1) == 0);

#[repr(C)]
struct FreeArea {
    header: Header,
    next: *mut FreeArea,
    prev: *mut FreeArea,
}

const HEADER: usize = size_of::<Header>();

/// The sentinels around a region's free area.
const REGION_OVERHEAD: usize = 2 * HEADER;

/// The largest chunk of a region, including its sentinels: its free area
/// still maps to a valid bucket. Upstream's `HEAP_LARGE_ALLOC_BYTES` plus its
/// grow overhead.
const MAX_CHUNK: usize = 1 << HEAP_ALLOC_VIRTUAL_BITS;

/// The smallest chunk worth adding: sentinels and one free area.
const MIN_CHUNK: usize = REGION_OVERHEAD + size_of::<FreeArea>();

/// The maximum size that `alloc` can allocate. Any larger size yields null.
pub(crate) const HEAP_MAX_ALLOC_SIZE: usize = (1 << 20) - HEADER;

/// An index into the free lists. Only `size_to_index_helper` makes one, and
/// the static assertions after it keep every index under `NUMBER_OF_BUCKETS`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Bucket(usize);

/// A bucket index and the size it rounds to.
struct SizeToIndex {
    bucket: Bucket,
    rounded_up: usize,
}

/// Operates in sizes that don't include the allocation header; that is, the
/// usable portion of a memory area. `size` is at least 8.
const fn size_to_index_helper(size: usize, adjust: usize, increment: usize) -> SizeToIndex {
    // First buckets are simply 8-spaced up to 128.
    if size <= 128 {
        let rounded_up = if size_of::<usize>() == 8 && size <= size_of::<FreeArea>() - HEADER {
            size_of::<FreeArea>() - HEADER
        } else {
            size
        };
        // No allocation is smaller than 8 bytes, so the first bucket is for 8
        // byte spaces (not including the header). For 64 bit, the free list
        // struct is 16 bytes larger than the header, so no allocation can be
        // smaller than that (otherwise how to free it), but we have empty 8
        // and 16 byte buckets for simplicity.
        return SizeToIndex { bucket: Bucket((size >> 3) - 1), rounded_up };
    }

    // We are going to go up to the next size to round up, but if we hit a
    // bucket size exactly we don't want to go up. By subtracting 8 here, we
    // will do the right thing (the carry propagates up for the round numbers
    // we are interested in). `adjust` is that subtraction, as an amount to
    // take away.
    let size = size - adjust;
    // After 128 the buckets are logarithmically spaced, every 16 up to 256,
    // every 32 up to 512 etc. This can be thought of as rows of 8 buckets.
    // E.g. 128-255 has 24 leading zeros on 32 bit, and we want row to be 4.
    let row = usize::BITS - 4 - size.leading_zeros();
    // For row 4 we want to shift down 4 bits.
    let column = (size >> row) & 7;
    let row_column = (((row as usize) << 3) | column) + increment;
    let rounded_up = (8 + (row_column & 7)) << (row_column >> 3);
    // We start with 15 buckets, 8, 16, 24, 32, 40, 48, 56, 64, 72, 80, 88, 96,
    // 104, 112, 120. Then we have row 4, sizes 128 and up, with the row-column
    // 8 and up.
    let answer = row_column + 15 - 32;
    debug_assert!(answer < NUMBER_OF_BUCKETS);
    SizeToIndex { bucket: Bucket(answer), rounded_up }
}

/// Round up size to next bucket when allocating. `size` is at least 1.
const fn size_to_index_allocating(size: usize) -> SizeToIndex {
    let rounded = size.next_multiple_of(8);
    size_to_index_helper(rounded, 8, 1)
}

/// Round down size to next bucket when freeing.
const fn size_to_index_freeing(size: usize) -> Bucket {
    size_to_index_helper(size, 0, 0).bucket
}

// The largest allocation maps to a bucket, and so does the free area of the
// largest chunk.
zr::static_assert!(size_to_index_allocating(HEAP_MAX_ALLOC_SIZE).bucket.0 < NUMBER_OF_BUCKETS);
zr::static_assert!(
    size_to_index_allocating(HEAP_MAX_ALLOC_SIZE).rounded_up + HEADER
        <= MAX_CHUNK - REGION_OVERHEAD
);
zr::static_assert!(
    size_to_index_freeing(MAX_CHUNK - REGION_OVERHEAD - HEADER).0 < NUMBER_OF_BUCKETS
);

fn tag_as_free(left: *mut Header) -> *mut Header {
    left.map_addr(|a| a | FREE_BIT)
}

fn untag(left: *mut Header) -> *mut Header {
    left.map_addr(|a| a & !FREE_BIT)
}

/// A header of a live heap: an area's, or a sentinel's.
///
/// The constructors take the proof that a pointer is such a header; the
/// methods then read and relink it in safe code. Every header a method returns
/// is one too, by the layout `add_chunk` sets up and every operation keeps: an
/// area's size reaches exactly to the next header of its chunk, its `left`
/// points at the header before it or is null at a left sentinel, and an area
/// other than a sentinel is at least `size_of::<FreeArea>()` bytes, so that it
/// can become free.
#[derive(Clone, Copy, PartialEq, Eq)]
struct HeaderRef(NonNull<Header>);

impl HeaderRef {
    /// The header before `payload`.
    ///
    /// # Safety
    ///
    /// `payload` is an allocation of a live heap.
    unsafe fn from_payload(payload: NonNull<u8>) -> Self {
        // SAFETY: an allocation's header lies right before it.
        Self(unsafe { payload.cast::<Header>().sub(1) })
    }

    /// Writes an allocated header of `size` bytes after `left` at `at`, and
    /// returns it.
    ///
    /// # Safety
    ///
    /// `at` is aligned to `HEAP_ALIGNMENT` and writable for a header, inside a
    /// chunk of a live heap where its layout expects one.
    unsafe fn write(at: *mut u8, size: usize, left: Option<HeaderRef>) -> Self {
        #[expect(
            clippy::cast_ptr_alignment,
            reason = "`at` is aligned to `HEAP_ALIGNMENT`, per the caller"
        )]
        let header = at.cast::<Header>();
        // SAFETY: the caller guarantees a header fits at `at`, which is inside
        // a chunk, so not null.
        unsafe {
            header.write(Header { left: Self::raw(left), size });
            Self(NonNull::new_unchecked(header))
        }
    }

    /// `left` as the raw pointer a header stores, untagged.
    fn raw(left: Option<HeaderRef>) -> *mut Header {
        left.map_or(ptr::null_mut(), |h| h.0.as_ptr())
    }

    /// The header's `left` field, tag included.
    fn left_raw(self) -> *mut Header {
        // SAFETY: a header of a live heap, per the type.
        unsafe { (*self.0.as_ptr()).left }
    }

    /// The size of the area in bytes, header included; 0 at a right sentinel.
    fn size(self) -> usize {
        // SAFETY: a header of a live heap, per the type.
        unsafe { (*self.0.as_ptr()).size }
    }

    /// Sets the area's size. The caller keeps the layout: the next header of
    /// the chunk lies `size` bytes on.
    fn set_size(self, size: usize) {
        // SAFETY: a header of a live heap, per the type.
        unsafe { (*self.0.as_ptr()).size = size }
    }

    /// Whether the area is free and in a bucket.
    fn is_free(self) -> bool {
        self.left_raw().addr() & FREE_BIT != 0
    }

    /// The header of the area before this one; `None` at a left sentinel.
    fn left(self) -> Option<HeaderRef> {
        NonNull::new(untag(self.left_raw())).map(Self)
    }

    /// Points `left` at `new_left`, keeping the free bit.
    fn set_left(self, new_left: Option<HeaderRef>) {
        let tag = self.left_raw().addr() & FREE_BIT;
        let left = Self::raw(new_left).map_addr(|a| (a & !FREE_BIT) | tag);
        // SAFETY: a header of a live heap, per the type.
        unsafe { (*self.0.as_ptr()).left = left }
    }

    /// Clears the free bit, as the area leaves its bucket.
    fn mark_allocated(self) {
        let left = untag(self.left_raw());
        // SAFETY: a header of a live heap, per the type.
        unsafe { (*self.0.as_ptr()).left = left }
    }

    /// The header of the area after this one. A right sentinel, of size 0, is
    /// its own right.
    fn right(self) -> HeaderRef {
        // SAFETY: an area's size reaches exactly to the next header of its chunk.
        Self(unsafe { self.0.byte_add(self.size()) })
    }

    /// The area as a free area; `None` unless it is free.
    fn as_free(self) -> Option<FreeAreaRef> {
        self.is_free().then_some(FreeAreaRef(self.0.cast()))
    }

    /// The area's first byte, where its header starts.
    fn as_bytes(self) -> *mut u8 {
        self.0.as_ptr().cast()
    }

    /// The byte `offset` into the area.
    fn at(self, offset: usize) -> *mut u8 {
        self.as_bytes().wrapping_add(offset)
    }

    /// The area's usable memory, after the header.
    fn payload(self) -> *mut u8 {
        self.at(HEADER)
    }
}

/// A free area of a live heap, in a bucket's list.
#[derive(Clone, Copy, PartialEq, Eq)]
struct FreeAreaRef(NonNull<FreeArea>);

impl FreeAreaRef {
    /// `area` as the raw pointer a list stores.
    fn raw(area: Option<FreeAreaRef>) -> *mut FreeArea {
        area.map_or(ptr::null_mut(), |a| a.0.as_ptr())
    }

    fn header(self) -> HeaderRef {
        HeaderRef(self.0.cast())
    }

    /// The next area in the bucket's list.
    fn next(self) -> Option<FreeAreaRef> {
        // SAFETY: a free area of a live heap, per the type.
        NonNull::new(unsafe { (*self.0.as_ptr()).next }).map(Self)
    }

    /// The previous area in the bucket's list; `None` at its head.
    fn prev(self) -> Option<FreeAreaRef> {
        // SAFETY: a free area of a live heap, per the type.
        NonNull::new(unsafe { (*self.0.as_ptr()).prev }).map(Self)
    }

    fn set_next(self, next: Option<FreeAreaRef>) {
        // SAFETY: a free area of a live heap, per the type.
        unsafe { (*self.0.as_ptr()).next = Self::raw(next) }
    }

    fn set_prev(self, prev: Option<FreeAreaRef>) {
        // SAFETY: a free area of a live heap, per the type.
        unsafe { (*self.0.as_ptr()).prev = Self::raw(prev) }
    }
}

/// A count of a heap's bytes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(C)]
pub struct HeapStats {
    /// Bytes the heap's regions hold, sentinels included.
    pub size: usize,
    /// Bytes in free areas, headers included.
    pub free: usize,
    /// The most bytes ever in use at once since the heap was built.
    pub peak_used: usize,
    /// Allocations that found no free area large enough.
    pub failures: usize,
}

zr::static_assert!(size_of::<HeapStats>() == 4 * size_of::<usize>());
zr::static_assert!(align_of::<HeapStats>() == align_of::<usize>());

/// A region's bounds, for `owns` and `reset`.
#[derive(Clone, Copy)]
struct Region {
    start: *mut u8,
    len: usize,
}

/// One heap: the free lists and the regions they carve.
pub(crate) struct CmpctHeap {
    /// Total bytes given to the heap, sentinels included.
    size: usize,
    /// Bytes of usable free space in the heap, headers included.
    remaining: usize,
    /// `remaining` right after the regions were added.
    capacity: usize,
    peak_used: usize,
    failures: usize,
    /// Free lists, bucketed by size. See `size_to_index_helper`.
    free_lists: [Option<FreeAreaRef>; NUMBER_OF_BUCKETS],
    /// Bitmask that tracks whether a given `free_lists` entry has any elements.
    /// See `set_free_list_bit` and `clear_free_list_bit`.
    free_list_bits: [u32; BUCKET_WORDS],
    regions: [Region; MAX_REGIONS],
    region_count: usize,
}

impl CmpctHeap {
    /// A heap with no memory: every allocation fails until `add_region`.
    pub(crate) const fn new() -> Self {
        Self {
            size: 0,
            remaining: 0,
            capacity: 0,
            peak_used: 0,
            failures: 0,
            free_lists: [None; NUMBER_OF_BUCKETS],
            free_list_bits: [0; BUCKET_WORDS],
            regions: [Region { start: ptr::null_mut(), len: 0 }; MAX_REGIONS],
            region_count: 0,
        }
    }

    /// The head of `bucket`'s list.
    fn list(&mut self, bucket: Bucket) -> &mut Option<FreeAreaRef> {
        &mut self.free_lists[bucket.0]
    }

    fn set_free_list_bit(&mut self, bucket: Bucket) {
        self.free_list_bits[bucket.0 >> 5] |= 1u32 << (31 - (bucket.0 & 0x1f));
    }

    fn clear_free_list_bit(&mut self, bucket: Bucket) {
        self.free_list_bits[bucket.0 >> 5] &= !(1u32 << (31 - (bucket.0 & 0x1f)));
    }

    /// The first non-empty bucket at or after `from`.
    fn find_nonempty_bucket(&self, from: Bucket) -> Option<Bucket> {
        let index = from.0;
        let mut mask = (1u32 << (31 - (index & 0x1f))) - 1;
        mask = mask * 2 + 1;
        mask &= self.free_list_bits[index >> 5];
        if mask != 0 {
            return Some(Bucket((index & !0x1f) + mask.leading_zeros() as usize));
        }
        let mut index = (index + 1).next_multiple_of(32);
        while index < NUMBER_OF_BUCKETS {
            let mask = self.free_list_bits[index >> 5];
            if mask != 0 {
                return Some(Bucket(index + mask.leading_zeros() as usize));
            }
            index += 32;
        }
        None
    }

    /// Puts the area at `header` in its bucket, at the head of the list, with
    /// its free bit set. Like every area other than a sentinel, it is at least
    /// `size_of::<FreeArea>()` bytes, so the list pointers fit after the header.
    fn make_free(&mut self, header: HeaderRef) {
        let size = header.size();
        let bucket = size_to_index_freeing(size - HEADER);
        self.set_free_list_bit(bucket);
        let old_head = *self.list(bucket);
        let area = FreeAreaRef(header.0.cast());
        // SAFETY: the area is at least a free area long, per the layout.
        unsafe {
            area.0.write(FreeArea {
                header: Header { left: tag_as_free(header.left_raw()), size },
                next: FreeAreaRef::raw(old_head),
                prev: ptr::null_mut(),
            });
        }
        if let Some(old_head) = old_head {
            old_head.set_prev(Some(area));
        }
        *self.list(bucket) = Some(area);
        self.remaining += size;
    }

    /// Takes `area` out of `bucket`.
    fn unlink_free(&mut self, area: FreeAreaRef, bucket: Bucket) {
        let size = area.header().size();
        assert!(self.remaining >= size, "{} >= {}", self.remaining, size);
        self.remaining -= size;
        let (next, prev) = (area.next(), area.prev());
        if *self.list(bucket) == Some(area) {
            *self.list(bucket) = next;
            if next.is_none() {
                self.clear_free_list_bit(bucket);
            }
        }
        if let Some(prev) = prev {
            prev.set_next(next);
        }
        if let Some(next) = next {
            next.set_prev(prev);
        }
    }

    fn unlink_free_unknown_bucket(&mut self, area: FreeAreaRef) {
        let bucket = size_to_index_freeing(area.header().size() - HEADER);
        self.unlink_free(area, bucket);
    }

    /// Lays out one chunk: sentinels around a single free area.
    ///
    /// # Safety
    ///
    /// `new_area` is aligned to `HEAP_ALIGNMENT` and writable for `size`
    /// bytes, with `MIN_CHUNK <= size <= MAX_CHUNK` a multiple of 8, and
    /// nothing else uses it while the heap does.
    unsafe fn add_chunk(&mut self, new_area: *mut u8, size: usize) {
        // SAFETY: the caller guarantees the chunk; its three headers lie at
        // its start, after the left sentinel, and at its end.
        let area = unsafe {
            // Set up the left sentinel. Its `left` field will not have FREE_BIT
            // set, stopping attempts to coalesce left.
            let left_sentinel = HeaderRef::write(new_area, HEADER, None);
            // Set up the usable memory area, which will be marked free.
            let area =
                HeaderRef::write(new_area.add(HEADER), size - REGION_OVERHEAD, Some(left_sentinel));
            // Set up the right sentinel. Its `left` field will not have FREE_BIT
            // set, stopping attempts to coalesce right.
            HeaderRef::write(new_area.add(size - HEADER), 0, Some(area));
            area
        };
        self.make_free(area);
        self.size += size;
    }

    /// Adds `len` bytes at `start` to the heap, in chunks of at most
    /// `MAX_CHUNK`. Returns false, adding nothing, when the heap already has
    /// `MAX_REGIONS` regions or the memory cannot hold one chunk.
    ///
    /// # Safety
    ///
    /// `start` is writable for `len` bytes, which nothing else uses while the
    /// heap does.
    pub(crate) unsafe fn add_region(&mut self, start: *mut u8, len: usize) -> bool {
        let skip = start.align_offset(HEAP_ALIGNMENT);
        let Some(len) = len.checked_sub(skip) else {
            return false;
        };
        let len = len & !(HEAP_ALIGNMENT - 1);
        if self.region_count == MAX_REGIONS || len < MIN_CHUNK {
            return false;
        }
        // SAFETY: `skip` is within the caller's `len` bytes.
        let start = unsafe { start.add(skip) };
        self.regions[self.region_count] = Region { start, len };
        self.region_count += 1;
        // Count only what this region adds: blocks still allocated elsewhere
        // will return their bytes to `remaining` when freed.
        let before = self.remaining;
        // SAFETY: the region is aligned and the caller's.
        unsafe { self.lay_out(start, len) };
        self.capacity += self.remaining - before;
        true
    }

    /// Splits `len` bytes at `start` into chunks.
    ///
    /// # Safety
    ///
    /// As for `add_region`, with `start` aligned and `len` a multiple of 8.
    unsafe fn lay_out(&mut self, start: *mut u8, len: usize) {
        let mut offset = 0;
        while len - offset >= MIN_CHUNK {
            let chunk = (len - offset).min(MAX_CHUNK);
            // SAFETY: the chunk lies within the region.
            unsafe { self.add_chunk(start.add(offset), chunk) };
            offset += chunk;
        }
    }

    /// Frees every allocation at once: each region becomes free areas again.
    pub(crate) fn reset(&mut self) {
        self.free_lists = [None; NUMBER_OF_BUCKETS];
        self.free_list_bits = [0; BUCKET_WORDS];
        self.size = 0;
        self.remaining = 0;
        self.peak_used = 0;
        self.failures = 0;
        for i in 0..self.region_count {
            let Region { start, len } = self.regions[i];
            // SAFETY: `add_region` checked the region, and resetting gives it
            // back to the heap whole.
            unsafe { self.lay_out(start, len) };
        }
        self.capacity = self.remaining;
    }

    /// The heap's regions, as start addresses and lengths.
    #[cfg(test)]
    pub(crate) fn regions(&self) -> impl Iterator<Item = (*mut u8, usize)> + '_ {
        self.regions[..self.region_count].iter().map(|r| (r.start, r.len))
    }

    pub(crate) fn stats(&self) -> HeapStats {
        HeapStats {
            size: self.size,
            free: self.remaining,
            peak_used: self.peak_used,
            failures: self.failures,
        }
    }

    /// Allocates `size` bytes aligned to `HEAP_ALIGNMENT`. `None` when `size`
    /// is 0 or above `HEAP_MAX_ALLOC_SIZE`, or no free area is large enough.
    pub(crate) fn alloc(&mut self, size: usize) -> Option<NonNull<u8>> {
        if size == 0 || size > HEAP_MAX_ALLOC_SIZE {
            return None;
        }
        let SizeToIndex { bucket: start_bucket, rounded_up } = size_to_index_allocating(size);
        let rounded_up = rounded_up + HEADER;

        let (bucket, head) = loop {
            let Some(bucket) = self.find_nonempty_bucket(start_bucket) else {
                // Upstream grows the heap here. A domain's heap is all the
                // memory it will ever have.
                self.failures += 1;
                #[cfg(feature = "forkpoint-coverage")]
                forkpoint::assert_sometimes!(
                    true,
                    "CmpctHeap::alloc: returns null when no free area fits"
                );
                return None;
            };
            match *self.list(bucket) {
                Some(head) if head.header().size() >= rounded_up => break (bucket, head),
                _ => {
                    // The bitmap says the bucket has a list, but its head is
                    // missing or smaller than the bucket's size: a heap whose
                    // memory was overwritten. Treat the bucket as empty and
                    // look on, rather than follow the head.
                    #[cfg(feature = "forkpoint")]
                    forkpoint::assert_unreachable!(
                        "CmpctHeap::alloc: a bucket with its bit set has a head of its size"
                    );
                    self.clear_free_list_bit(bucket);
                }
            }
        };
        let header = head.header();
        let left_over = header.size() - rounded_up;
        // We can't carve off the rest for a new free space if it's smaller
        // than the free-list linked structure. We also don't carve it off
        // if it's less than 1.6% the size of the allocation. This is to
        // avoid small long-lived allocations being placed right next to
        // large allocations, hindering coalescing.
        if left_over >= size_of::<FreeArea>() && left_over > (size >> 6) {
            #[cfg(feature = "forkpoint-coverage")]
            forkpoint::assert_sometimes!(true, "CmpctHeap::alloc: splits a larger free area");
            let right = header.right();
            self.unlink_free(head, bucket);
            // SAFETY: `rounded_up` bytes into the area, a multiple of 8, with
            // `left_over` bytes to the area's end: inside this chunk, where
            // the split puts a header.
            let tail = unsafe { HeaderRef::write(header.at(rounded_up), left_over, Some(header)) };
            header.set_size(rounded_up);
            self.make_free(tail);
            right.set_left(Some(tail));
        } else {
            #[cfg(feature = "forkpoint-coverage")]
            forkpoint::assert_sometimes!(true, "CmpctHeap::alloc: hands out a whole free area");
            self.unlink_free(head, bucket);
        }
        header.mark_allocated();
        self.peak_used = self.peak_used.max(self.capacity - self.remaining);
        NonNull::new(header.payload())
    }

    /// Allocates `size` bytes aligned to `alignment`, a power of two.
    pub(crate) fn memalign(&mut self, alignment: usize, size: usize) -> Option<NonNull<u8>> {
        if size == 0 {
            return None;
        }
        if alignment <= HEAP_ALIGNMENT {
            return self.alloc(size);
        }
        // The aligned block must be able to become a free area once freed: on
        // 64 bit, a free area is larger than a header and 8 bytes.
        let padded_size = size
            .max(size_of::<FreeArea>() - HEADER)
            .checked_add(alignment)?
            .checked_add(size_of::<FreeArea>())?;
        let unaligned = self.alloc(padded_size)?;
        let mask = alignment - 1;
        let payload_int = (unaligned.addr().get() + size_of::<FreeArea>() + mask) & !mask;
        let payload = unaligned.as_ptr().with_addr(payload_int);
        // SAFETY: `unaligned` is an allocation of this heap, just made.
        let block = unsafe { HeaderRef::from_payload(unaligned) };
        let left_over = payload_int - unaligned.addr().get();
        // SAFETY: `payload` lies at least a free area's size into the
        // allocation `unaligned`, which has room for `size` bytes after it, so
        // a header fits before it, aligned. The part before `payload` becomes
        // its own allocation and is freed, and so is what lies past `size`:
        // upstream left that tail in the block.
        let header = unsafe {
            HeaderRef::write(payload.wrapping_sub(HEADER), block.size() - left_over, Some(block))
        };
        let right = block.right();
        block.set_size(left_over);
        right.set_left(Some(header));
        self.release(block);
        self.trim(header, size);
        NonNull::new(payload)
    }

    /// The size a block needs, header included, to hold `size` bytes and to
    /// become a free area once freed.
    fn block_size(size: usize) -> usize {
        (HEADER + size.next_multiple_of(8)).max(size_of::<FreeArea>())
    }

    /// Gives the end of the allocated block at `header` back to the heap,
    /// keeping room for `size` bytes, when the end can form a free area.
    fn trim(&mut self, header: HeaderRef, size: usize) {
        let needed = Self::block_size(size);
        let Some(left_over) = header.size().checked_sub(needed) else {
            return;
        };
        if left_over < size_of::<FreeArea>() {
            return;
        }
        let right = header.right();
        // SAFETY: `needed` bytes into the block, a multiple of 8, with
        // `left_over` bytes to the block's end: inside this chunk, where the
        // split puts a header.
        let tail = unsafe { HeaderRef::write(header.at(needed), left_over, Some(header)) };
        right.set_left(Some(tail));
        header.set_size(needed);
        // The tail coalesces with a free right neighbor.
        self.release(tail);
    }

    /// Resizes the allocation at `payload` to hold `size` bytes without moving
    /// it: shrinking gives its end back to the heap, and growing takes in a
    /// free right neighbor. Returns false, changing nothing, when the block
    /// cannot grow in place or `size` is 0 or above `HEAP_MAX_ALLOC_SIZE`.
    ///
    /// # Safety
    ///
    /// `payload` is a live allocation of this heap.
    pub(crate) unsafe fn resize_in_place(&mut self, payload: NonNull<u8>, size: usize) -> bool {
        if size == 0 || size > HEAP_MAX_ALLOC_SIZE {
            return false;
        }
        let needed = Self::block_size(size);
        // SAFETY: the caller guarantees a live allocation.
        let header = unsafe { HeaderRef::from_payload(payload) };
        if header.size() < needed {
            let right = header.right();
            let Some(right_area) = right.as_free() else {
                return false;
            };
            if header.size() + right.size() < needed {
                return false;
            }
            self.unlink_free_unknown_bucket(right_area);
            let right_right = right.right();
            header.set_size(header.size() + right.size());
            right_right.set_left(Some(header));
        }
        self.trim(header, size);
        // After the trim: the block held the neighbor whole only until then.
        self.peak_used = self.peak_used.max(self.capacity - self.remaining);
        true
    }

    /// Whether `payload` is the start of a live allocation of this heap. Reads
    /// only headers inside the heap's regions, and takes constant time: the
    /// block's header must be consistent with both of its neighbors.
    pub(crate) fn owns(&self, payload: *const u8) -> bool {
        let addr = payload.addr();
        if addr & (HEAP_ALIGNMENT - 1) != 0 || addr < HEADER {
            return false;
        }
        let header_addr = addr - HEADER;
        let Some(region) = self.regions[..self.region_count]
            .iter()
            .find(|r| header_addr.wrapping_sub(r.start.addr()) < r.len)
        else {
            return false;
        };
        let (start, end) = (region.start.addr(), region.start.addr() + region.len);
        // Every header below is read through the region's own pointer, after
        // checking that it lies inside the region.
        let read = |at: usize| -> Option<(usize, usize)> {
            if at & (HEAP_ALIGNMENT - 1) != 0 || at < start || at.checked_add(HEADER)? > end {
                return None;
            }
            #[expect(
                clippy::cast_ptr_alignment,
                reason = "`at` is a multiple of `HEAP_ALIGNMENT`, checked above"
            )]
            let header = region.start.with_addr(at).cast::<Header>();
            // SAFETY: a whole header lies in the region, which the heap owns.
            let h = unsafe { &*header };
            Some((h.left.addr(), h.size))
        };
        let Some((left, size)) = read(header_addr) else {
            return false;
        };
        if left & FREE_BIT != 0 || left == 0 || size < size_of::<FreeArea>() || size & 7 != 0 {
            return false;
        }
        // The right neighbor points back, and the left neighbor ends here.
        let Some((right_left, _)) = header_addr.checked_add(size).and_then(read) else {
            return false;
        };
        let Some((_, left_size)) = read(left) else {
            return false;
        };
        right_left & !FREE_BIT == header_addr && left.checked_add(left_size) == Some(header_addr)
    }

    /// The bytes the allocation at `payload` can hold.
    ///
    /// # Safety
    ///
    /// `payload` is an allocation of this heap.
    pub(crate) unsafe fn usable_size(payload: NonNull<u8>) -> usize {
        // SAFETY: the caller guarantees an allocation, which has a header.
        let header = unsafe { HeaderRef::from_payload(payload) };
        header.size() - HEADER
    }

    /// Frees the allocation at `payload`, coalescing it with free neighbors.
    ///
    /// # Safety
    ///
    /// `payload` is an allocation of this heap not yet freed.
    pub(crate) unsafe fn free(&mut self, payload: NonNull<u8>) {
        // SAFETY: the caller guarantees an allocation of this heap.
        let header = unsafe { HeaderRef::from_payload(payload) };
        debug_assert!(!header.is_free()); // Double free!
        assert!(header.size() > HEADER, "got {} min {}", header.size(), HEADER);
        self.release(header);
    }

    /// Frees the allocated area at `header`, coalescing it with free neighbors.
    /// Its left and right neighbors are areas or sentinels of the same chunk.
    fn release(&mut self, header: HeaderRef) {
        let size = header.size();
        let right = header.right();
        if let Some(left_area) = header.left().and_then(HeaderRef::as_free) {
            let left = left_area.header();
            self.unlink_free_unknown_bucket(left_area);
            if let Some(right_area) = right.as_free() {
                #[cfg(feature = "forkpoint-coverage")]
                forkpoint::assert_sometimes!(
                    true,
                    "CmpctHeap::free: coalesces with both neighbors"
                );
                // Coalesce both sides.
                self.unlink_free_unknown_bucket(right_area);
                right.right().set_left(Some(left));
                left.set_size(left.size() + size + right.size());
            } else {
                #[cfg(feature = "forkpoint-coverage")]
                forkpoint::assert_sometimes!(
                    true,
                    "CmpctHeap::free: coalesces with the left neighbor"
                );
                // Coalesce only left.
                right.set_left(Some(left));
                left.set_size(left.size() + size);
            }
            self.make_free(left);
        } else if let Some(right_area) = right.as_free() {
            #[cfg(feature = "forkpoint-coverage")]
            forkpoint::assert_sometimes!(
                true,
                "CmpctHeap::free: coalesces with the right neighbor"
            );
            // Coalesce only right.
            right.right().set_left(Some(header));
            self.unlink_free_unknown_bucket(right_area);
            header.set_size(size + right.size());
            self.make_free(header);
        } else {
            #[cfg(feature = "forkpoint-coverage")]
            forkpoint::assert_sometimes!(
                true,
                "CmpctHeap::free: has no free neighbor to coalesce with"
            );
            self.make_free(header);
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use std::vec::Vec;

    use super::*;

    /// The C-style calls upstream's tests make: null for no allocation.
    impl CmpctHeap {
        fn alloc_raw(&mut self, size: usize) -> *mut u8 {
            self.alloc(size).map_or(ptr::null_mut(), NonNull::as_ptr)
        }

        fn memalign_raw(&mut self, alignment: usize, size: usize) -> *mut u8 {
            self.memalign(alignment, size).map_or(ptr::null_mut(), NonNull::as_ptr)
        }

        /// Like C's `free`, ignores null.
        ///
        /// # Safety
        ///
        /// As for `free`, unless `payload` is null.
        unsafe fn free_raw(&mut self, payload: *mut u8) {
            if let Some(payload) = NonNull::new(payload) {
                // SAFETY: the caller's guarantee.
                unsafe { self.free(payload) }
            }
        }

        /// # Safety
        ///
        /// As for `resize_in_place`, and `payload` is not null.
        unsafe fn resize_in_place_raw(&mut self, payload: *mut u8, size: usize) -> bool {
            // SAFETY: the caller's guarantee.
            unsafe { self.resize_in_place(NonNull::new_unchecked(payload), size) }
        }

        /// # Safety
        ///
        /// As for `usable_size`, and `payload` is not null.
        unsafe fn usable_size_raw(payload: *mut u8) -> usize {
            // SAFETY: the caller's guarantee.
            unsafe { Self::usable_size(NonNull::new_unchecked(payload)) }
        }
    }

    /// A heap over `N` bytes of its own. The memory is held as a raw pointer,
    /// so that moving the `TestHeap` does not invalidate the heap's pointers
    /// into it.
    struct TestHeap<const N: usize> {
        heap: CmpctHeap,
        memory: *mut [u64],
    }

    impl<const N: usize> TestHeap<N> {
        fn new() -> Self {
            let memory = std::boxed::Box::into_raw(std::vec![0u64; N / 8].into_boxed_slice());
            let mut heap = CmpctHeap::new();
            // SAFETY: the memory is the heap's until `drop` frees it.
            assert!(unsafe { heap.add_region(memory.cast(), N) });
            Self { heap, memory }
        }
    }

    impl<const N: usize> Drop for TestHeap<N> {
        fn drop(&mut self) {
            // SAFETY: `memory` came from `Box::into_raw`, and the heap goes too.
            drop(unsafe { std::boxed::Box::from_raw(self.memory) });
        }
    }

    // Miri tracks every byte the stress tests write and read back, which takes
    // it tens of minutes at full size; it runs them smaller, over the same
    // paths.

    /// The heap the fill-and-free stress tests fill.
    const STRESS_HEAP: usize = if cfg!(miri) { 32 * 1024 } else { 256 * 1024 };
    /// The largest block they ask for.
    const STRESS_MAX_SIZE: usize = if cfg!(miri) { 512 } else { 4096 };
    /// The heap, the steps, and the largest block of the random memalign test.
    const RANDOM_HEAP: usize = if cfg!(miri) { 256 * 1024 } else { 1 << 20 };
    const RANDOM_STEPS: usize = if cfg!(miri) { 2048 } else { 32768 };
    const RANDOM_MAX_SIZE: usize = if cfg!(miri) { 4096 } else { 32768 };

    /// A deterministic generator, as upstream seeds its own with `kRandomSeed`.
    struct Lcg(u64);

    impl Lcg {
        fn next(&mut self, bound: usize) -> usize {
            self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            ((self.0 >> 33) as usize) % bound
        }
    }

    // Upstream `cmpct_test_buckets`.
    #[test]
    fn buckets() {
        // Check for the 8-spaced buckets up to 128.
        for i in 1..=128 {
            // Round up when allocating.
            let SizeToIndex { bucket: Bucket(bucket), rounded_up } = size_to_index_allocating(i);
            let expected = (i.next_multiple_of(8) >> 3) - 1;
            assert_eq!(bucket, expected);
            assert_eq!(rounded_up % 8, 0);
            assert!(rounded_up >= i);
            if i >= size_of::<FreeArea>() - HEADER {
                // Once we get above the size of the free area struct (4 words),
                // we won't round up much for these small size.
                assert!(rounded_up - i < 8);
            }
            // Only rounded sizes are freed.
            if i & 7 == 0 {
                // Up to size 128 we have exact buckets for each multiple of 8.
                assert_eq!(Bucket(bucket), size_to_index_freeing(i));
            }
        }
        let mut bucket_base = 7;
        let mut j = 16;
        while j < 1024 {
            // Note the "<=", which ensures that we test the powers of 2 twice
            // to ensure that both ways of calculating the bucket number match.
            for i in j * 8..=j * 16 {
                // Round up to j multiple in this range when allocating.
                let SizeToIndex { bucket: Bucket(bucket), rounded_up } =
                    size_to_index_allocating(i);
                let expected = bucket_base + i.div_ceil(j);
                assert_eq!(bucket, expected);
                assert_eq!(rounded_up % j, 0);
                assert!(rounded_up >= i);
                assert!(rounded_up - i < j);
                // Only 8-rounded sizes are freed or chopped off the end of a
                // free area when allocating.
                if i & 7 == 0 {
                    // When freeing, if we don't hit the size of the bucket
                    // precisely, we have to put the free space into a smaller
                    // bucket, because the buckets have entries that will
                    // always be big enough for the corresponding allocation
                    // size (so we don't have to traverse the free chains to
                    // find a big enough one).
                    if i % j == 0 {
                        assert_eq!(Bucket(bucket), size_to_index_freeing(i));
                    } else {
                        assert_eq!(Bucket(bucket - 1), size_to_index_freeing(i));
                    }
                }
            }
            j *= 2;
            bucket_base += 8;
        }
    }

    // Upstream `cmpct_test_get_back_newly_freed`, over the sizes a 1 MiB heap holds.
    #[test]
    fn get_back_newly_freed() {
        let mut t = TestHeap::<{ 1 << 20 }>::new();
        let heap = &mut t.heap;
        let mut helper = |size: usize| {
            let allocated = heap.alloc_raw(size);
            if allocated.is_null() {
                return;
            }
            let allocated2 = heap.alloc_raw(8);
            // SAFETY: both are allocations of `heap`.
            unsafe {
                heap.free_raw(allocated);
                let allocated3 = heap.alloc_raw(size);
                // To avoid churn and fragmentation we would want to get the
                // newly freed memory back again when we allocate the same size
                // shortly after.
                assert_eq!(allocated3, allocated);
                heap.free_raw(allocated2);
                heap.free_raw(allocated3);
            }
        };
        let mut increment = 16;
        let mut i = 128;
        while i <= 1 << 18 {
            let mut j = i;
            while j < i * 2 {
                helper(i - 8);
                helper(i);
                helper(i + 1);
                j += increment;
            }
            i *= 2;
            increment *= 2;
        }
        for i in 1024..=2048 {
            helper(i);
        }
    }

    // Upstream `ZeroAllocIsNull`.
    #[test]
    fn zero_alloc_is_null() {
        let mut t = TestHeap::<4096>::new();
        assert!(t.heap.alloc_raw(0).is_null());
        assert_eq!(t.heap.stats().failures, 0);
    }

    // Upstream `NullCanBeFreed`. The heap takes only non-null pointers; null
    // stops at the C entry points, which `test/c/rivet_heap/malloc.c` covers.
    #[test]
    fn null_can_be_freed() {
        let mut t = TestHeap::<4096>::new();
        // SAFETY: null is always accepted.
        unsafe { t.heap.free_raw(ptr::null_mut()) };
    }

    // Upstream's double-free check, a debug assertion there too.
    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "header.is_free()")]
    fn a_double_free_fails_its_assertion() {
        let mut t = TestHeap::<4096>::new();
        let allocated = t.heap.alloc_raw(16);
        assert!(!allocated.is_null());
        // SAFETY: `allocated` is an allocation of the heap; the second free is
        // the double free the assertion catches before touching the heap.
        unsafe {
            t.heap.free_raw(allocated);
            t.heap.free_raw(allocated);
        }
    }

    // Upstream `HeapIsProperlyInitialized`. There is no cached memory to check.
    #[test]
    fn heap_is_properly_initialized() {
        let t = TestHeap::<4096>::new();
        let stats = t.heap.stats();
        assert_eq!(stats.size, 4096);
        assert_eq!(stats.free, 4096 - REGION_OVERHEAD);
        assert_eq!(stats.peak_used, 0);
    }

    #[derive(Clone, Copy)]
    enum FreeOrder {
        Chronological,
        ReverseChronological,
        Random,
    }

    const ORDERS: [FreeOrder; 3] =
        [FreeOrder::Chronological, FreeOrder::ReverseChronological, FreeOrder::Random];

    /// Upstream's `RandomAllocator`: random sizes, filled to catch overlap with
    /// the heap's own structures, then freed in `order`. Allocates until the
    /// heap is full, where upstream allocates until it has grown 10 times.
    fn allocate_until_full(heap: &mut CmpctHeap, aligned: bool, order: FreeOrder) {
        const ALLOC_FILL: u8 = 0x51;
        let mut rng = Lcg(101);
        let mut allocated = Vec::new();
        let mut misses = 0;
        while misses < 8 {
            let size = 1 + rng.next(STRESS_MAX_SIZE);
            let alignment =
                if aligned { size_of::<usize>() << rng.next(9) } else { HEAP_ALIGNMENT };
            let p = if aligned { heap.memalign_raw(alignment, size) } else { heap.alloc_raw(size) };
            if p.is_null() {
                misses += 1;
                continue;
            }
            assert_eq!(p.addr() % alignment, 0);
            assert!(heap.owns(p));
            // SAFETY: `p` holds at least `size` bytes.
            unsafe { p.write_bytes(ALLOC_FILL, size) };
            allocated.push((p, size));
        }
        match order {
            FreeOrder::Chronological => {}
            FreeOrder::ReverseChronological => allocated.reverse(),
            FreeOrder::Random => {
                for i in (1..allocated.len()).rev() {
                    allocated.swap(i, rng.next(i + 1));
                }
            }
        }
        for (p, size) in allocated {
            // SAFETY: each is an allocation of `heap` holding `size` bytes,
            // freed once.
            unsafe {
                // A block that overlapped another, or the heap's own headers,
                // would have lost its fill.
                for i in 0..size {
                    assert_eq!(*p.add(i), ALLOC_FILL);
                }
                assert!(heap.owns(p));
                heap.free_raw(p);
            }
        }
        // Everything coalesced back into the one free area.
        assert_eq!(heap.stats().free, heap.stats().size - REGION_OVERHEAD);
    }

    // Upstream `CanAllocAndFree`.
    #[test]
    fn can_alloc_and_free() {
        for order in ORDERS {
            let mut t = TestHeap::<STRESS_HEAP>::new();
            allocate_until_full(&mut t.heap, false, order);
        }
    }

    // Upstream `CanMemalignAndFree`.
    #[test]
    fn can_memalign_and_free() {
        for order in ORDERS {
            let mut t = TestHeap::<STRESS_HEAP>::new();
            allocate_until_full(&mut t.heap, true, order);
        }
    }

    // Upstream `SizedFree`, through `usable_size`, which `free_sized` checks.
    #[test]
    fn sized_free() {
        let mut t = TestHeap::<4096>::new();
        let p = t.heap.alloc_raw(1000);
        assert!(!p.is_null());
        // SAFETY: `p` is an allocation of the heap.
        unsafe {
            assert!(CmpctHeap::usable_size_raw(p) >= 1000);
            t.heap.free_raw(p);
        }
    }

    // Upstream `SizedFreeFromMemalign`.
    #[test]
    fn sized_free_from_memalign() {
        let mut t = TestHeap::<4096>::new();
        let p = t.heap.memalign_raw(128, 1);
        assert!(!p.is_null());
        assert_eq!(p.addr() % 128, 0);
        // SAFETY: `p` is an allocation of the heap.
        unsafe {
            assert!(CmpctHeap::usable_size_raw(p) >= 1);
            t.heap.free_raw(p);
        }
    }

    // Upstream `LargeAllocsAreNull`.
    #[test]
    fn large_allocs_are_null() {
        let mut t = TestHeap::<{ 2 << 20 }>::new();
        let p = t.heap.alloc_raw(HEAP_MAX_ALLOC_SIZE);
        assert!(!p.is_null());
        // SAFETY: `p` is an allocation of the heap.
        unsafe { t.heap.free_raw(p) };
        assert!(t.heap.alloc_raw(HEAP_MAX_ALLOC_SIZE + 1).is_null());
        assert_eq!(t.heap.stats().failures, 0);
    }

    // Upstream `cmpct_test`'s random memalign and free over 16 slots.
    #[test]
    fn random_memalign_and_free() {
        let mut t = TestHeap::<RANDOM_HEAP>::new();
        let heap = &mut t.heap;
        let mut rng = Lcg(1);
        let mut slots = [ptr::null_mut(); 16];
        for _ in 0..RANDOM_STEPS {
            let index = rng.next(16);
            // SAFETY: every slot holds null or a live allocation of `heap`.
            unsafe { heap.free_raw(slots[index]) };
            let align = 1 << rng.next(8);
            slots[index] = heap.memalign_raw(align, rng.next(RANDOM_MAX_SIZE));
            assert_eq!(slots[index].addr() % align, 0);
        }
        for p in slots {
            // SAFETY: as above.
            unsafe { heap.free_raw(p) };
        }
        assert_eq!(heap.stats().free, heap.stats().size - REGION_OVERHEAD);
    }

    /// Allocates 64-byte blocks until the heap has no room for another.
    fn fill(heap: &mut CmpctHeap) -> usize {
        let mut count = 0;
        while !heap.alloc_raw(64).is_null() {
            count += 1;
        }
        count
    }

    #[test]
    fn exhausting_the_heap_returns_null_and_counts_a_failure() {
        let mut t = TestHeap::<4096>::new();
        let count = fill(&mut t.heap);
        assert!(count > 0);
        let stats = t.heap.stats();
        assert_eq!(stats.failures, 1);
        assert!(stats.free < 64 + HEADER);
        assert_eq!(stats.peak_used, stats.size - REGION_OVERHEAD - stats.free);
    }

    #[test]
    fn reset_frees_everything() {
        let mut t = TestHeap::<4096>::new();
        let count = fill(&mut t.heap);
        t.heap.reset();
        let stats = t.heap.stats();
        assert_eq!(stats.free, 4096 - REGION_OVERHEAD);
        assert_eq!((stats.peak_used, stats.failures), (0, 0));
        assert_eq!(fill(&mut t.heap), count);
    }

    #[test]
    fn a_region_added_while_a_block_is_live_counts_only_its_own_bytes() {
        let mut t = TestHeap::<4096>::new();
        let p = t.heap.alloc_raw(1000);
        assert!(!p.is_null());
        let second = TestHeap::<4096>::new();
        // SAFETY: the second test heap's memory outlives this heap's use of it;
        // its own heap is never used.
        unsafe {
            assert!(t.heap.add_region(second.memory.cast(), 4096));
            t.heap.free_raw(p);
        }
        // Freeing the block made `remaining` exceed a capacity taken after it.
        let q = t.heap.alloc_raw(8);
        assert!(!q.is_null());
        let stats = t.heap.stats();
        assert_eq!(stats.size, 8192);
        assert!(stats.peak_used < stats.size);
    }

    /// The largest block a fresh heap of `N` bytes hands out.
    fn largest_block<const N: usize>(heap: &mut CmpctHeap) -> *mut u8 {
        (8..N).rev().step_by(8).map(|size| heap.alloc_raw(size)).find(|p| !p.is_null()).unwrap()
    }

    #[test]
    fn resize_in_place_shrinks_the_only_block_of_a_full_heap() {
        let mut t = TestHeap::<4096>::new();
        let p = largest_block::<4096>(&mut t.heap);
        // Bucket rounding leaves a little room after the largest block: fill it.
        while !t.heap.alloc_raw(8).is_null() {}
        assert!(t.heap.alloc_raw(64).is_null());
        // SAFETY: `p` is a live allocation of the heap.
        unsafe {
            p.write_bytes(0x3c, 64);
            assert!(t.heap.resize_in_place_raw(p, 64));
            for i in 0..64 {
                assert_eq!(*p.add(i), 0x3c);
            }
        }
        assert!(t.heap.owns(p));
        assert!(!t.heap.alloc_raw(64).is_null());
    }

    #[test]
    fn resize_in_place_grows_into_a_free_right_neighbor_only() {
        let mut t = TestHeap::<8192>::new();
        let p = t.heap.alloc_raw(64);
        // SAFETY: `p` and `q` are live allocations of the heap.
        unsafe {
            // The rest of the heap is free on the right.
            assert!(t.heap.resize_in_place_raw(p, 1000));
            assert!(CmpctHeap::usable_size_raw(p) >= 1000);
            let q = t.heap.alloc_raw(64);
            // Now `q` is allocated on the right.
            assert!(!t.heap.resize_in_place_raw(p, 4000));
            assert!(t.heap.owns(p) && t.heap.owns(q));
            t.heap.free_raw(q);
            t.heap.free_raw(p);
        }
        assert_eq!(t.heap.stats().free, 8192 - REGION_OVERHEAD);
    }

    #[test]
    fn resize_in_place_counts_only_what_it_keeps_toward_the_peak() {
        let mut t = TestHeap::<8192>::new();
        let p = t.heap.alloc_raw(8);
        // SAFETY: `p` is a live allocation of the heap.
        unsafe {
            // The rest of the heap is one free block on the right: growing
            // takes it whole, then trims the excess back off. 64 bytes need a
            // larger block than 8 on every target, so the block does grow.
            assert!(CmpctHeap::usable_size_raw(p) < 64);
            assert!(t.heap.resize_in_place_raw(p, 64));
        }
        let stats = t.heap.stats();
        // Nothing was ever in use but `p`, so the peak is what `p` holds now,
        // not the free block it passed through.
        assert_eq!(stats.peak_used, stats.size - REGION_OVERHEAD - stats.free);
        // SAFETY: as above.
        unsafe { t.heap.free_raw(p) };
    }

    #[test]
    fn owns_rejects_a_block_whose_left_is_misaligned() {
        let mut t = TestHeap::<4096>::new();
        let p = t.heap.alloc_raw(64);
        let (start, _) = t.heap.regions().next().unwrap();
        // A header forged inside `p`'s payload: a plausible size, and a `left`
        // inside the region but off a header boundary. `owns` must reject it
        // without reading a header there.
        #[expect(clippy::cast_ptr_alignment, reason = "a deliberately misaligned header pointer")]
        let left = start.wrapping_add(2).cast::<Header>();
        #[expect(clippy::cast_ptr_alignment, reason = "`p` is aligned to `HEAP_ALIGNMENT`")]
        let forged = p.cast::<Header>();
        // SAFETY: `p` holds 64 writable bytes, and a header is 16 of them.
        unsafe { forged.write(Header { left, size: 32 }) };
        assert!(!t.heap.owns(p.wrapping_add(HEADER)));
        // SAFETY: `p` is a live allocation.
        unsafe { t.heap.free_raw(p) };
    }

    #[test]
    fn alloc_skips_a_bucket_whose_bit_is_set_without_a_head() {
        let mut t = TestHeap::<4096>::new();
        // Claim a small bucket has a free area while its list is empty, as a
        // heap whose memory was overwritten might. The allocation must come
        // from the larger free area behind it, not fail.
        let small = size_to_index_allocating(64).bucket;
        t.heap.set_free_list_bit(small);
        assert!(t.heap.list(small).is_none());
        let p = t.heap.alloc_raw(64);
        assert!(!p.is_null() && t.heap.owns(p));
        assert_eq!(t.heap.stats().failures, 0);
        // SAFETY: `p` is a live allocation.
        unsafe { t.heap.free_raw(p) };
    }

    #[test]
    fn owns_accepts_only_the_start_of_a_live_block() {
        let mut t = TestHeap::<4096>::new();
        let p = t.heap.alloc_raw(100);
        let q = t.heap.alloc_raw(100);
        assert!(t.heap.owns(p) && t.heap.owns(q));
        // Inside a block, misaligned, in the free tail, and outside the heap.
        assert!(!t.heap.owns(p.wrapping_add(8)));
        assert!(!t.heap.owns(p.wrapping_add(16)));
        assert!(!t.heap.owns(p.wrapping_add(1)));
        assert!(!t.heap.owns(q.wrapping_add(512)));
        assert!(!t.heap.owns(ptr::null()));
        let outside = [0u64; 4];
        assert!(!t.heap.owns(outside.as_ptr().cast::<u8>().wrapping_add(16)));
        // SAFETY: `p` is a live allocation, freed once.
        unsafe { t.heap.free_raw(p) };
        assert!(!t.heap.owns(p));
    }

    #[test]
    fn memalign_gives_back_the_tail_past_the_aligned_block() {
        let mut t = TestHeap::<{ 64 * 1024 }>::new();
        let p = t.heap.memalign_raw(4096, 8);
        assert_eq!(p.addr() % 4096, 0);
        // Only the block and its header stay in use, not the padding.
        assert!(t.heap.stats().free >= 64 * 1024 - REGION_OVERHEAD - 64);
        // SAFETY: `p` is a live allocation, freed once.
        unsafe { t.heap.free_raw(p) };
        assert_eq!(t.heap.stats().free, 64 * 1024 - REGION_OVERHEAD);
    }

    #[test]
    fn memalign_blocks_of_every_small_size_free_cleanly() {
        let mut t = TestHeap::<{ 64 * 1024 }>::new();
        let mut blocks = Vec::new();
        for size in 1..=40 {
            for alignment in [16, 32, 64, 128] {
                let p = t.heap.memalign_raw(alignment, size);
                assert_eq!(p.addr() % alignment, 0);
                // SAFETY: `p` holds `size` bytes.
                unsafe { p.write_bytes(size as u8, size) };
                blocks.push((p, size));
            }
        }
        for (p, size) in blocks {
            // SAFETY: each is a live allocation holding `size` bytes, freed once.
            unsafe {
                for i in 0..size {
                    assert_eq!(*p.add(i), size as u8);
                }
                t.heap.free_raw(p);
            }
        }
        assert_eq!(t.heap.stats().free, 64 * 1024 - REGION_OVERHEAD);
    }

    #[test]
    fn a_region_larger_than_a_chunk_is_split() {
        let t = TestHeap::<{ MAX_CHUNK + 4096 }>::new();
        assert_eq!(t.heap.stats().size, MAX_CHUNK + 4096);
        assert_eq!(t.heap.stats().free, MAX_CHUNK + 4096 - 2 * REGION_OVERHEAD);
    }

    #[test]
    fn add_region_aligns_and_refuses_what_it_cannot_use() {
        let mut memory = [0u64; 64];
        let mut heap = CmpctHeap::new();
        let base = memory.as_mut_ptr().cast::<u8>();
        // SAFETY: every range lies in `memory`, which outlives `heap`.
        unsafe {
            assert!(!heap.add_region(base, MIN_CHUNK - 1));
            assert!(heap.add_region(base.add(1), 256));
            let (start, len) = heap.regions().next().unwrap();
            assert_eq!(start, base.add(8));
            assert_eq!(len, 248);
        }
    }
}
