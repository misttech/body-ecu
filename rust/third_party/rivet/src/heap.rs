// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

//! Heaps for memory domains, over the allocator in `support::cmpctmalloc`.
//!
//! Each heap owns the memory it is given and never grows, so it cannot hand
//! out memory outside it, and every call takes bounded time. A kernel that
//! partitions RAM into domains gives each domain its own heap, as an
//! [`Arena`] from Rust or with `RIVET_HEAP_ARENA` and `rivet_heap_init` from
//! C, and tells rivet which heap is running through [`rivet_heap_hooks`]. The
//! C `malloc` family then allocates from the running domain's heap. Without
//! domains, `malloc` uses one default heap built from `rivet_heap_add`.
//!
//! Safe Rust in a domain reaches its heap only through the [`DomainHeap`] it
//! was handed; C code and anything unsafe is kept in bounds by the MPU, which
//! the kernel programs from [`DomainHeap::bounds`].

// Sizes here come from the caller: arithmetic on them is checked, or wraps on
// purpose, so no input can overflow into a wrong bound.
#![deny(clippy::arithmetic_side_effects)]

use core::alloc::Layout;
use core::cell::{Cell, UnsafeCell};
use core::marker::PhantomData;
use core::mem::{MaybeUninit, size_of};
use core::ops::Range;
use core::ptr::{self, NonNull};
use core::sync::atomic::{AtomicBool, AtomicPtr, Ordering};

use crate::support::cmpctmalloc::{CmpctHeap, HEAP_ALIGNMENT};

pub use crate::support::cmpctmalloc::HeapStats;

/// Bytes of an arena that hold its heap's own state rather than allocations:
/// the bucket lists, the bitmap, the counters, and the region's sentinels.
/// `RIVET_HEAP_OVERHEAD` in `include/rivet_heap.h` is the same expression.
pub const HEAP_OVERHEAD: usize = 144 * size_of::<usize>() + 32;

/// Bytes at the start of an arena that hold the heap's own state, aligned for
/// the allocations that follow.
pub(crate) const HEAP_STATE: usize = size_of::<Heap>().next_multiple_of(HEAP_ALIGNMENT);

/// Bytes of [`HEAP_OVERHEAD`] left after the state: the sentinels around the
/// region. A `HEAP_OVERHEAD` too small for the state fails to compile here.
pub(crate) const HEAP_SENTINELS: usize = HEAP_OVERHEAD - HEAP_STATE;

zr::static_assert!(HEAP_SENTINELS >= size_of::<[usize; 4]>());

/// One heap; C sees it as the opaque `struct rivet_heap`.
#[repr(transparent)]
pub struct Heap(UnsafeCell<CmpctHeap>);

// SAFETY: every access to the inner heap goes through `Heap::with`, which holds
// the kernel's lock for the heap when one is registered. Without a lock hook,
// the kernel has promised that only one thread uses the heap.
unsafe impl Sync for Heap {}

/// The C name of [`Heap`].
#[allow(non_camel_case_types)]
pub type rivet_heap = Heap;

/// The calls a kernel makes available to rivet's heaps, registered with
/// `rivet_heap_set_hooks`. Each may be null.
#[repr(C)]
#[allow(non_camel_case_types)]
pub struct rivet_heap_hooks {
    /// The running domain's heap, or null for the default heap.
    pub current: Option<unsafe extern "C" fn() -> *mut rivet_heap>,
    /// Takes the heap's lock: a domain mutex, or a critical section.
    pub lock: Option<unsafe extern "C" fn(*mut rivet_heap)>,
    /// Releases what `lock` took.
    pub unlock: Option<unsafe extern "C" fn(*mut rivet_heap)>,
}

zr::static_assert!(size_of::<rivet_heap_hooks>() == size_of::<[usize; 3]>());
zr::static_assert!(align_of::<rivet_heap_hooks>() == align_of::<usize>());

static HOOKS: AtomicPtr<rivet_heap_hooks> = AtomicPtr::new(ptr::null_mut());

/// The heap `malloc` uses when there are no domains, or when the running code
/// belongs to none. `rivet_heap_add` gives it memory.
pub(crate) static DEFAULT_HEAP: Heap = Heap::new();

/// Registers the kernel's calls, replacing any registered before. A call
/// already in progress may still use the hooks it started with, so hooks
/// that are replaced must stay valid too, as a `&'static` does.
pub fn set_hooks(hooks: &'static rivet_heap_hooks) {
    HOOKS.store(ptr::from_ref(hooks).cast_mut(), Ordering::Release);
}

fn hooks() -> Option<&'static rivet_heap_hooks> {
    // SAFETY: `HOOKS` is null or was stored from a `&'static`.
    unsafe { HOOKS.load(Ordering::Acquire).as_ref() }
}

impl Heap {
    /// A heap with no memory.
    pub const fn new() -> Self {
        Self(UnsafeCell::new(CmpctHeap::new()))
    }

    /// The running domain's heap, or the default heap.
    pub(crate) fn current() -> &'static Heap {
        let current = hooks()
            .and_then(|h| h.current)
            // SAFETY: the kernel's hook takes no arguments and returns a heap
            // pointer or null.
            .map_or(ptr::null_mut(), |current| unsafe { current() });
        // SAFETY: a heap the kernel names is one it built with `Arena::take`
        // or `rivet_heap_init`, which lives as long as the program.
        unsafe { current.as_ref() }.unwrap_or(&DEFAULT_HEAP)
    }

    /// Runs `f` on the heap, holding the heap's lock if the kernel gave one.
    pub(crate) fn with<R>(&self, f: impl FnOnce(&mut CmpctHeap) -> R) -> R {
        let raw = self.as_raw();
        let hooks = hooks();
        if let Some(lock) = hooks.and_then(|h| h.lock) {
            // SAFETY: the kernel's hook takes this heap.
            unsafe { lock(raw) };
        }
        // SAFETY: the lock, or the kernel's promise of a single user, makes
        // this the only reference to the heap until `f` returns.
        let result = f(unsafe { &mut *self.0.get() });
        if let Some(unlock) = hooks.and_then(|h| h.unlock) {
            // SAFETY: as above.
            unsafe { unlock(raw) };
        }
        result
    }

    /// The heap, without its lock.
    ///
    /// # Safety
    ///
    /// Nothing else uses the heap until the reference is dropped: it is being
    /// built, before anyone else can reach it.
    #[allow(clippy::mut_from_ref)]
    pub(crate) unsafe fn unlocked(&self) -> &mut CmpctHeap {
        // SAFETY: the caller guarantees exclusive use.
        unsafe { &mut *self.0.get() }
    }

    /// The heap as C sees it.
    pub fn as_raw(&self) -> *mut rivet_heap {
        ptr::from_ref(self).cast_mut()
    }

    /// The heap's counters.
    pub fn stats(&self) -> HeapStats {
        self.with(|h| h.stats())
    }
}

impl Default for Heap {
    fn default() -> Self {
        Self::new()
    }
}

/// The memory of an arena: `BYTES` for the heap, and room for the sentinels
/// that bound it.
#[repr(C, align(8))]
struct ArenaMemory<const BYTES: usize> {
    bytes: [MaybeUninit<u8>; BYTES],
    sentinels: [MaybeUninit<usize>; 4],
}

/// A domain's heap and its memory, sized at compile time.
///
/// Declare one with [`rivet_heap_arena!`](crate::rivet_heap_arena), and hand
/// [`Arena::take`]'s result to the domain. Its section must start zeroed, as
/// `.bss` does: `take` reads a flag that starts false.
#[repr(C, align(8))]
pub struct Arena<const BYTES: usize> {
    heap: Heap,
    taken: AtomicBool,
    memory: UnsafeCell<ArenaMemory<BYTES>>,
}

// SAFETY: the memory is only reached through the heap, after `take` handed
// the heap to one owner.
unsafe impl<const BYTES: usize> Sync for Arena<BYTES> {}

impl<const BYTES: usize> Arena<BYTES> {
    /// An arena whose heap is built on the first `take`.
    pub const fn new() -> Self {
        const {
            assert!(BYTES & 7 == 0, "an arena's size is a multiple of 8");
            assert!(BYTES >= 64, "an arena holds at least 64 bytes");
        };
        Self {
            heap: Heap::new(),
            taken: AtomicBool::new(false),
            memory: UnsafeCell::new(ArenaMemory {
                bytes: [MaybeUninit::uninit(); BYTES],
                sentinels: [MaybeUninit::uninit(); 4],
            }),
        }
    }

    /// Builds the heap and returns its only handle; `None` after the first call.
    pub fn take(&'static self) -> Option<DomainHeap> {
        if self.taken.swap(true, Ordering::AcqRel) {
            return None;
        }
        // SAFETY: the flag was false, so no handle exists and nothing else can
        // reach the heap or its memory.
        let added = unsafe {
            let h = self.heap.unlocked();
            *h = CmpctHeap::new();
            h.add_region(self.memory.get().cast(), size_of::<ArenaMemory<BYTES>>())
        };
        // At least 64 bytes always hold one chunk; a heap that could not use
        // its memory would fail every allocation, so hand out nothing.
        if !added {
            return None;
        }
        let start = ptr::from_ref(self);
        Some(DomainHeap {
            heap: &self.heap,
            // One past the arena is an address, as the end of any object is.
            bounds: start.addr()..start.wrapping_add(1).addr(),
            _not_sync: PhantomData,
        })
    }
}

impl<const BYTES: usize> Default for Arena<BYTES> {
    fn default() -> Self {
        Self::new()
    }
}

/// A domain's heap: the only handle to it, moved to the domain.
///
/// It is `Send`, so it can move into the domain's thread, but not `Sync`
/// or `Clone`, so safe code shares it only by reference within one thread.
pub struct DomainHeap {
    heap: &'static Heap,
    bounds: Range<usize>,
    _not_sync: PhantomData<Cell<()>>,
}

impl DomainHeap {
    /// Allocates memory for `layout`, or `None` when it is empty or the heap
    /// has no room.
    pub fn alloc(&self, layout: Layout) -> Option<NonNull<u8>> {
        self.heap.with(|h| h.memalign(layout.align(), layout.size()))
    }

    /// Frees an allocation.
    ///
    /// # Safety
    ///
    /// `ptr` came from this heap's [`DomainHeap::alloc`] and is not used again.
    pub unsafe fn free(&self, ptr: NonNull<u8>) {
        // SAFETY: the caller guarantees an allocation of this heap.
        self.heap.with(|h| unsafe { h.free(ptr) });
    }

    /// Frees every allocation at once, after the domain faulted. Taking the
    /// handle by value leaves safe code nothing that points into the old heap.
    pub fn reset(self) -> DomainHeap {
        self.heap.with(CmpctHeap::reset);
        self
    }

    /// The addresses the heap and its memory occupy, for the kernel's MPU
    /// region for the domain.
    pub fn bounds(&self) -> Range<usize> {
        self.bounds.clone()
    }

    /// The heap's counters.
    pub fn stats(&self) -> HeapStats {
        self.heap.stats()
    }

    /// The heap as C sees it, for the kernel's `current` hook.
    pub fn as_raw(&self) -> *mut rivet_heap {
        self.heap.as_raw()
    }
}

/// Declares `static NAME: Arena<BYTES>` in the linker section
/// `.bss.rivet_heap.NAME`, the section `RIVET_HEAP_ARENA` uses from C.
///
/// Firmware's usual `*(.bss .bss.*)` rule zeroes and places it with the rest of
/// `.bss`. A kernel that gives a domain its own RAM range places
/// `*(.bss.rivet_heap.NAME)` there instead, ahead of that rule, and covers it
/// with one MPU region. The section is only set on a bare-metal target: host
/// builds keep their own section naming.
///
/// ```ignore
/// rivet_libc::rivet_heap_arena!(UI, 16 * 1024);
///
/// let ui = UI.take().unwrap(); // hand to the UI domain
/// ```
#[macro_export]
macro_rules! rivet_heap_arena {
    ($(#[$attr:meta])* $vis:vis $name:ident, $bytes:expr $(,)?) => {
        $(#[$attr])*
        #[cfg_attr(
            target_os = "none",
            unsafe(link_section = concat!(".bss.rivet_heap.", stringify!($name)))
        )]
        $vis static $name: $crate::heap::Arena<{ $bytes }> = $crate::heap::Arena::new();
    };
}
