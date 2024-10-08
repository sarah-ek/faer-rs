use crate::{utils::slice, Entity, Index, Shape, ShapeIdx, SignedIndex, Unbind};
use core::{fmt, marker::PhantomData, ops::Range};
use faer_entity::*;
use generativity::Guard;
use reborrow::*;

type Invariant<'a> = fn(&'a ()) -> &'a ();

/// Splits a range into two segments.
#[derive(Copy, Clone)]
pub struct Partition<'head, 'tail, 'n> {
    /// Size of the first half.
    pub head: Dim<'head>,
    /// Size of the second half.
    pub tail: Dim<'tail>,
    __marker: PhantomData<Invariant<'n>>,
}

impl<'head, 'tail, 'n> Partition<'head, 'tail, 'n> {
    /// Returns the midpoint of the partition.
    #[inline]
    pub const fn midpoint(&self) -> IdxInc<'n> {
        unsafe { IdxInc::new_unbound(self.head.unbound) }
    }
}

/// Lifetime branded length
/// # Safety
/// The type's safety invariant is that all instances of this type with the same lifetime
/// correspond to the same length.
#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct Dim<'n> {
    unbound: usize,
    __marker: PhantomData<Invariant<'n>>,
}
impl PartialEq for Dim<'_> {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        equator::debug_assert!(self.unbound == other.unbound);
        true
    }
}
impl Eq for Dim<'_> {}

impl PartialOrd for Dim<'_> {
    #[inline(always)]
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        equator::debug_assert!(self.unbound == other.unbound);
        Some(core::cmp::Ordering::Equal)
    }
}
impl Ord for Dim<'_> {
    #[inline(always)]
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        equator::debug_assert!(self.unbound == other.unbound);
        core::cmp::Ordering::Equal
    }
}

/// Lifetime branded index.
/// # Safety
/// The type's safety invariant is that all instances of this type are valid indices for
/// [`Dim<'n>`] and less than or equal to `I::Signed::MAX`.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Idx<'n, I: Index = usize> {
    unbound: I,
    __marker: PhantomData<Invariant<'n>>,
}

/// Lifetime branded partition index.
/// # Safety
/// The type's safety invariant is that all instances of this type are valid partition places for
/// [`Dim<'n>`] and less than or equal to `I::Signed::MAX`.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct IdxInc<'n, I: Index = usize> {
    unbound: I,
    __marker: PhantomData<Invariant<'n>>,
}

impl fmt::Debug for Dim<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.unbound.fmt(f)
    }
}
impl<I: Index> fmt::Debug for Idx<'_, I> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.unbound.fmt(f)
    }
}
impl<I: Index> fmt::Debug for IdxInc<'_, I> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.unbound.fmt(f)
    }
}
impl<I: Index> fmt::Debug for MaybeIdx<'_, I> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.unbound.to_signed() >= I::Signed::truncate(0) {
            self.unbound.fmt(f)
        } else {
            f.write_str("None")
        }
    }
}

impl<'n, I: Index> PartialEq<Dim<'n>> for Idx<'n, I> {
    #[inline(always)]
    fn eq(&self, other: &Dim<'n>) -> bool {
        equator::debug_assert!(self.unbound.zx() < other.unbound);

        false
    }
}
impl<'n, I: Index> PartialOrd<Dim<'n>> for Idx<'n, I> {
    #[inline(always)]
    fn partial_cmp(&self, other: &Dim<'n>) -> Option<core::cmp::Ordering> {
        equator::debug_assert!(self.unbound.zx() < other.unbound);

        Some(core::cmp::Ordering::Less)
    }
}

impl<'n, I: Index> PartialEq<Dim<'n>> for IdxInc<'n, I> {
    #[inline(always)]
    fn eq(&self, other: &Dim<'n>) -> bool {
        equator::debug_assert!(self.unbound.zx() <= other.unbound);

        self.unbound.zx() == other.unbound
    }
}

impl<'n, I: Index> PartialOrd<Dim<'n>> for IdxInc<'n, I> {
    #[inline(always)]
    fn partial_cmp(&self, other: &Dim<'n>) -> Option<core::cmp::Ordering> {
        equator::debug_assert!(self.unbound.zx() <= other.unbound);

        Some(if self.unbound.zx() == other.unbound {
            core::cmp::Ordering::Equal
        } else {
            core::cmp::Ordering::Less
        })
    }
}

impl<'n> Dim<'n> {
    /// Create new branded value with an arbitrary brand.
    /// # Safety
    /// See struct safety invariant.
    #[inline(always)]
    pub const unsafe fn new_unbound(dim: usize) -> Self {
        Self {
            unbound: dim,
            __marker: PhantomData,
        }
    }

    /// Create new branded value with a unique brand.
    #[inline(always)]
    pub fn new(dim: usize, guard: Guard<'n>) -> Self {
        _ = guard;
        Self {
            unbound: dim,
            __marker: PhantomData,
        }
    }

    /// Returns the unconstrained value.
    #[inline(always)]
    pub const fn unbound(self) -> usize {
        self.unbound
    }

    /// Partitions `self` into two segments as specifiedd by the midpoint.
    #[inline]
    pub const fn partition<'head, 'tail>(
        self,
        midpoint: IdxInc<'n>,
        head: Guard<'head>,
        tail: Guard<'tail>,
    ) -> Partition<'head, 'tail, 'n> {
        _ = (head, tail);
        unsafe {
            Partition {
                head: Dim::new_unbound(midpoint.unbound),
                tail: Dim::new_unbound(self.unbound - midpoint.unbound),
                __marker: PhantomData,
            }
        }
    }

    /// Returns an iterator over the indices between `0` and `self`.
    #[inline]
    pub fn indices(self) -> impl Clone + ExactSizeIterator + DoubleEndedIterator<Item = Idx<'n>> {
        (0..self.unbound).map(|i| unsafe { Idx::new_unbound(i) })
    }
}

impl<'n, I: Index> Idx<'n, I> {
    /// Create new branded value with an arbitrary brand.
    /// # Safety
    /// See struct safety invariant.
    #[inline(always)]
    pub const unsafe fn new_unbound(idx: I) -> Self {
        Self {
            unbound: idx,
            __marker: PhantomData,
        }
    }

    /// Create new branded value with the same brand as `dim`.
    /// # Safety
    /// The behavior is undefined unless `idx < dim` and `idx <= I::Signed::MAX`.
    #[inline(always)]
    pub unsafe fn new_unchecked(idx: I, dim: Dim<'n>) -> Self {
        equator::debug_assert!(all(
            idx.zx() < dim.unbound,
            idx <= I::from_signed(I::Signed::MAX),
        ));

        Self {
            unbound: idx,
            __marker: PhantomData,
        }
    }

    /// Create new branded value with the same brand as `dim`.
    /// # Panics
    /// Panics unless `idx < dim` and `idx <= I::Signed::MAX`.
    #[inline(always)]
    pub fn new_checked(idx: I, dim: Dim<'n>) -> Self {
        equator::assert!(all(
            idx.zx() < dim.unbound,
            idx <= I::from_signed(I::Signed::MAX),
        ));

        Self {
            unbound: idx,
            __marker: PhantomData,
        }
    }

    /// Returns the unconstrained value.
    #[inline(always)]
    pub const fn unbound(self) -> I {
        self.unbound
    }

    /// Zero-extends the internal value into a `usize`.
    #[inline(always)]
    pub fn zx(self) -> Idx<'n> {
        Idx {
            unbound: self.unbound.zx(),
            __marker: PhantomData,
        }
    }
}

impl<'n, I: Index> IdxInc<'n, I> {
    /// Create new branded value with an arbitrary brand.
    /// # Safety
    /// See struct safety invariant.
    #[inline(always)]
    pub const fn zero() -> Self {
        Self {
            unbound: unsafe { core::mem::zeroed() },
            __marker: PhantomData,
        }
    }

    /// Create new branded value with an arbitrary brand.
    /// # Safety
    /// See struct safety invariant.
    #[inline(always)]
    pub const unsafe fn new_unbound(idx: I) -> Self {
        Self {
            unbound: idx,
            __marker: PhantomData,
        }
    }

    /// Create new branded value with the same brand as `dim`.
    /// # Safety
    /// The behavior is undefined unless `idx <= dim`.
    #[inline(always)]
    pub unsafe fn new_unchecked(idx: I, dim: Dim<'n>) -> Self {
        equator::debug_assert!(all(
            idx.zx() <= dim.unbound,
            idx <= I::from_signed(I::Signed::MAX),
        ));

        Self {
            unbound: idx,
            __marker: PhantomData,
        }
    }

    /// Create new branded value with the same brand as `dim`.
    /// # Panics
    /// Panics unless `idx <= dim`.
    #[inline(always)]
    pub fn new_checked(idx: I, dim: Dim<'n>) -> Self {
        equator::assert!(all(
            idx.zx() <= dim.unbound,
            idx <= I::from_signed(I::Signed::MAX),
        ));

        Self {
            unbound: idx,
            __marker: PhantomData,
        }
    }

    /// Returns the unconstrained value.
    #[inline(always)]
    pub const fn unbound(self) -> I {
        self.unbound
    }

    /// Zero-extends the internal value into a `usize`.
    #[inline(always)]
    pub fn zx(self) -> IdxInc<'n> {
        IdxInc {
            unbound: self.unbound.zx(),
            __marker: PhantomData,
        }
    }
}

impl<'n> IdxInc<'n> {
    /// Returns an iterator over the indices between `self` and `to`.
    #[inline]
    pub fn to(
        self,
        upper: IdxInc<'n>,
    ) -> impl Clone + ExactSizeIterator + DoubleEndedIterator<Item = Idx<'n>> {
        (self.unbound..upper.unbound).map(|i| unsafe { Idx::new_unbound(i) })
    }

    /// Returns an iterator over the indices between `self` and `to`.
    #[inline]
    pub fn range_to(
        self,
        upper: IdxInc<'n>,
    ) -> impl Clone + ExactSizeIterator + DoubleEndedIterator<Item = Idx<'n>> {
        (self.unbound..upper.unbound).map(|i| unsafe { Idx::new_unbound(i) })
    }
}

impl Unbind for Dim<'_> {
    #[inline(always)]
    unsafe fn new_unbound(idx: usize) -> Self {
        Self::new_unbound(idx)
    }

    #[inline(always)]
    fn unbound(self) -> usize {
        self.unbound
    }
}
impl<I: Index> Unbind<I> for Idx<'_, I> {
    #[inline(always)]
    unsafe fn new_unbound(idx: I) -> Self {
        Self::new_unbound(idx)
    }

    #[inline(always)]
    fn unbound(self) -> I {
        self.unbound
    }
}
impl<I: Index> Unbind<I> for IdxInc<'_, I> {
    #[inline(always)]
    unsafe fn new_unbound(idx: I) -> Self {
        Self::new_unbound(idx)
    }

    #[inline(always)]
    fn unbound(self) -> I {
        self.unbound
    }
}

impl<'dim> ShapeIdx for Dim<'dim> {
    type Idx<I: Index> = Idx<'dim, I>;
    type IdxInc<I: Index> = IdxInc<'dim, I>;
}

impl<'dim> Shape for Dim<'dim> {}

impl<'n, I: Index> From<Idx<'n, I>> for IdxInc<'n, I> {
    #[inline(always)]
    fn from(value: Idx<'n, I>) -> Self {
        Self {
            unbound: value.unbound,
            __marker: PhantomData,
        }
    }
}

impl<'size> Dim<'size> {
    /// Check that the index is bounded by `self`, or panic otherwise.
    #[track_caller]
    #[inline]
    pub fn check<I: Index>(self, idx: I) -> Idx<'size, I> {
        Idx::new_checked(idx, self)
    }

    /// Check that the index is bounded by `self`, or return `None` otherwise.
    #[inline]
    pub fn try_check<I: Index>(self, idx: I) -> Option<Idx<'size, I>> {
        if idx.zx() < self.unbound() {
            Some(unsafe { Idx::new_unbound(idx) })
        } else {
            None
        }
    }
}

impl<'n> Idx<'n> {
    /// Truncate `self` to a smaller type `I`.
    pub fn truncate<I: Index>(self) -> Idx<'n, I> {
        unsafe { Idx::new_unbound(I::truncate(self.unbound())) }
    }
}

impl<'n, I: Index> Idx<'n, I> {
    /// Returns the index, bounded inclusively by the value tied to `'n`.
    #[inline]
    pub const fn to_inclusive(self) -> IdxInc<'n, I> {
        unsafe { IdxInc::new_unbound(self.unbound()) }
    }
    /// Returns the next index, bounded inclusively by the value tied to `'n`.
    #[inline]
    pub fn next(self) -> IdxInc<'n, I> {
        unsafe { IdxInc::new_unbound(self.unbound() + I::truncate(1)) }
    }

    /// Assert that the values of `slice` are all bounded by `size`.
    #[track_caller]
    #[inline]
    pub fn from_slice_mut_checked<'a>(slice: &'a mut [I], size: Dim<'n>) -> &'a mut [Idx<'n, I>] {
        Self::from_slice_ref_checked(slice, size);
        unsafe { &mut *(slice as *mut _ as *mut _) }
    }

    /// Assume that the values of `slice` are all bounded by the value tied to `'n`.
    #[track_caller]
    #[inline]
    pub unsafe fn from_slice_mut_unchecked<'a>(slice: &'a mut [I]) -> &'a mut [Idx<'n, I>] {
        unsafe { &mut *(slice as *mut _ as *mut _) }
    }

    /// Assert that the values of `slice` are all bounded by `size`.
    #[track_caller]
    pub fn from_slice_ref_checked<'a>(slice: &'a [I], size: Dim<'n>) -> &'a [Idx<'n, I>] {
        for &idx in slice {
            Self::new_checked(idx, size);
        }
        unsafe { &*(slice as *const _ as *const _) }
    }

    /// Assume that the values of `slice` are all bounded by the value tied to `'n`.
    #[track_caller]
    #[inline]
    pub unsafe fn from_slice_ref_unchecked<'a>(slice: &'a [I]) -> &'a [Idx<'n, I>] {
        unsafe { &*(slice as *const _ as *const _) }
    }
}

/// `I` value smaller than the size corresponding to the lifetime `'n`, or `None`.
#[derive(Copy, Clone, PartialEq, Eq)]
#[repr(transparent)]
pub struct MaybeIdx<'n, I: Index = usize> {
    unbound: I,
    __marker: PhantomData<Invariant<'n>>,
}

impl<'n, I: Index> MaybeIdx<'n, I> {
    /// Returns an index value.
    #[inline]
    pub fn from_index(idx: Idx<'n, I>) -> Self {
        unsafe { Self::new_unbound(idx.unbound()) }
    }
    /// Returns a `None` value.
    #[inline]
    pub fn none() -> Self {
        unsafe { Self::new_unbound(I::truncate(usize::MAX)) }
    }

    /// Returns a constrained index value if `idx` is nonnegative, `None` otherwise.
    #[inline]
    pub fn new_checked(idx: I::Signed, size: Dim<'n>) -> Self {
        assert!((idx.sx() as isize) < size.unbound() as isize);
        Self {
            unbound: I::from_signed(idx),
            __marker: PhantomData,
        }
    }

    /// Returns a constrained index value if `idx` is nonnegative, `None` otherwise.
    #[inline]
    pub unsafe fn new_unchecked(idx: I::Signed, size: Dim<'n>) -> Self {
        debug_assert!((idx.sx() as isize) < size.unbound() as isize);
        Self {
            unbound: I::from_signed(idx),
            __marker: PhantomData,
        }
    }

    /// Returns a constrained index value if `idx` is nonnegative, `None` otherwise.
    #[inline]
    pub unsafe fn new_unbound(idx: I) -> Self {
        Self {
            unbound: idx,
            __marker: PhantomData,
        }
    }

    /// Returns the inner value.
    #[inline]
    pub fn unbound(self) -> I {
        self.unbound
    }

    /// Returns the index if available, or `None` otherwise.
    #[inline]
    pub fn idx(self) -> Option<Idx<'n, I>> {
        if self.unbound.to_signed() >= I::Signed::truncate(0) {
            Some(unsafe { Idx::new_unbound(self.unbound()) })
        } else {
            None
        }
    }

    /// Sign extend the value.
    #[inline]
    pub fn sx(self) -> MaybeIdx<'n> {
        unsafe { MaybeIdx::new_unbound(self.unbound.to_signed().sx()) }
    }

    /// Assert that the values of `slice` are all bounded by `size`.
    #[track_caller]
    #[inline]
    pub fn from_slice_mut_checked<'a>(
        slice: &'a mut [I::Signed],
        size: Dim<'n>,
    ) -> &'a mut [MaybeIdx<'n, I>] {
        Self::from_slice_ref_checked(slice, size);
        unsafe { &mut *(slice as *mut _ as *mut _) }
    }

    /// Assume that the values of `slice` are all bounded by the value tied to `'n`.
    #[track_caller]
    #[inline]
    pub unsafe fn from_slice_mut_unchecked<'a>(
        slice: &'a mut [I::Signed],
    ) -> &'a mut [MaybeIdx<'n, I>] {
        unsafe { &mut *(slice as *mut _ as *mut _) }
    }

    /// Assert that the values of `slice` are all bounded by `size`.
    #[track_caller]
    pub fn from_slice_ref_checked<'a>(
        slice: &'a [I::Signed],
        size: Dim<'n>,
    ) -> &'a [MaybeIdx<'n, I>] {
        for &idx in slice {
            Self::new_checked(idx, size);
        }
        unsafe { &*(slice as *const _ as *const _) }
    }

    /// Convert a constrained slice to an unconstrained one.
    #[track_caller]
    pub fn as_slice_ref<'a>(slice: &'a [MaybeIdx<'n, I>]) -> &'a [I::Signed] {
        unsafe { &*(slice as *const _ as *const _) }
    }

    /// Assume that the values of `slice` are all bounded by the value tied to `'n`.
    #[track_caller]
    #[inline]
    pub unsafe fn from_slice_ref_unchecked<'a>(slice: &'a [I::Signed]) -> &'a [MaybeIdx<'n, I>] {
        unsafe { &*(slice as *const _ as *const _) }
    }
}

impl core::ops::Deref for Dim<'_> {
    type Target = usize;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.unbound
    }
}
impl<I: Index> core::ops::Deref for Idx<'_, I> {
    type Target = I;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.unbound
    }
}
impl<I: Index> core::ops::Deref for MaybeIdx<'_, I> {
    type Target = I::Signed;
    #[inline]
    fn deref(&self) -> &Self::Target {
        bytemuck::cast_ref(&self.unbound)
    }
}
impl<I: Index> core::ops::Deref for IdxInc<'_, I> {
    type Target = I;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.unbound
    }
}

/// Array of length equal to the value tied to `'n`.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Array<'n, T> {
    __marker: PhantomData<Invariant<'n>>,
    unbound: [T],
}

/// Immutable array group of length equal to the value tied to `'n`.
pub struct ArrayGroup<'n, 'a, E: Entity> {
    __marker: PhantomData<Invariant<'n>>,
    unbound: slice::SliceGroup<'a, E>,
}
/// Mutable array group of length equal to the value tied to `'n`.
pub struct ArrayGroupMut<'n, 'a, E: Entity> {
    __marker: PhantomData<Invariant<'n>>,
    unbound: slice::SliceGroupMut<'a, E>,
}

impl<E: Entity> core::fmt::Debug for ArrayGroup<'_, '_, E> {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.unbound.fmt(f)
    }
}
impl<E: Entity> core::fmt::Debug for ArrayGroupMut<'_, '_, E> {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.unbound.fmt(f)
    }
}

impl<E: Entity> Copy for ArrayGroup<'_, '_, E> {}
impl<E: Entity> Clone for ArrayGroup<'_, '_, E> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<'short, 'n, 'a, E: Entity> reborrow::ReborrowMut<'short> for ArrayGroup<'n, 'a, E> {
    type Target = ArrayGroup<'n, 'short, E>;

    #[inline]
    fn rb_mut(&'short mut self) -> Self::Target {
        *self
    }
}

impl<'short, 'n, 'a, E: Entity> reborrow::Reborrow<'short> for ArrayGroup<'n, 'a, E> {
    type Target = ArrayGroup<'n, 'short, E>;

    #[inline]
    fn rb(&'short self) -> Self::Target {
        *self
    }
}

impl<'short, 'n, 'a, E: Entity> reborrow::ReborrowMut<'short> for ArrayGroupMut<'n, 'a, E> {
    type Target = ArrayGroupMut<'n, 'short, E>;

    #[inline]
    fn rb_mut(&'short mut self) -> Self::Target {
        ArrayGroupMut {
            __marker: PhantomData,
            unbound: self.unbound.rb_mut(),
        }
    }
}

impl<'short, 'n, 'a, E: Entity> reborrow::Reborrow<'short> for ArrayGroupMut<'n, 'a, E> {
    type Target = ArrayGroup<'n, 'short, E>;

    #[inline]
    fn rb(&'short self) -> Self::Target {
        ArrayGroup {
            __marker: PhantomData,
            unbound: self.unbound.rb(),
        }
    }
}

impl<'n, 'a, E: Entity> ArrayGroupMut<'n, 'a, E> {
    /// Returns an array group with length after checking that its length matches
    /// the value tied to `'n`.
    #[inline]
    pub fn new(slice: GroupFor<E, &'a mut [E::Unit]>, len: Dim<'n>) -> Self {
        let slice = slice::SliceGroupMut::<'_, E>::new(slice);
        assert!(slice.len() == len.unbound());
        ArrayGroupMut {
            __marker: PhantomData,
            unbound: slice,
        }
    }

    /// Returns the unconstrained slice.
    #[inline]
    pub fn into_slice(self) -> GroupFor<E, &'a mut [E::Unit]> {
        self.unbound.into_inner()
    }

    /// Returns a subslice at from the range start to its end.
    #[inline]
    pub fn subslice(self, range: Range<IdxInc<'n>>) -> GroupFor<E, &'a mut [E::Unit]> {
        unsafe {
            slice::SliceGroupMut::<'_, E>::new(self.into_slice())
                .subslice_unchecked(range.start.unbound()..range.end.unbound())
                .into_inner()
        }
    }

    /// Read the element at position `j`.
    #[inline]
    pub fn read(&self, j: Idx<'n>) -> E {
        self.rb().read(j)
    }

    /// Write `value` to the location at position `j`.
    #[inline]
    pub fn write(&mut self, j: Idx<'n>, value: E) {
        unsafe {
            slice::SliceGroupMut::new(self.rb_mut().into_slice())
                .write_unchecked(j.unbound(), value)
        }
    }
}

impl<'n, 'a, E: Entity> ArrayGroup<'n, 'a, E> {
    /// Returns an array group with length after checking that its length matches
    /// the value tied to `'n`.
    #[inline]
    pub fn new(slice: GroupFor<E, &'a [E::Unit]>, len: Dim<'n>) -> Self {
        let slice = slice::SliceGroup::<'_, E>::new(slice);
        assert!(slice.rb().len() == len.unbound());
        ArrayGroup {
            __marker: PhantomData,
            unbound: slice,
        }
    }

    /// Returns the unconstrained slice.
    #[inline]
    pub fn into_slice(self) -> GroupFor<E, &'a [E::Unit]> {
        self.unbound.into_inner()
    }

    /// Returns a subslice at from the range start to its end.
    #[inline]
    pub fn subslice(self, range: Range<IdxInc<'n>>) -> GroupFor<E, &'a [E::Unit]> {
        unsafe {
            slice::SliceGroup::<'_, E>::new(self.into_slice())
                .subslice_unchecked(range.start.unbound()..range.end.unbound())
                .into_inner()
        }
    }

    /// Read the element at position `j`.
    #[inline]
    pub fn read(&self, j: Idx<'n>) -> E {
        unsafe { slice::SliceGroup::new(self.into_slice()).read_unchecked(j.unbound()) }
    }
}

impl<'n, T> Array<'n, T> {
    /// Returns a constrained array after checking that its length matches `size`.
    #[inline]
    #[track_caller]
    pub fn from_ref<'a>(slice: &'a [T], size: Dim<'n>) -> &'a Self {
        assert!(slice.len() == size.unbound());
        unsafe { &*(slice as *const [T] as *const Self) }
    }

    /// Returns a constrained array after checking that its length matches `size`.
    #[inline]
    #[track_caller]
    pub fn from_mut<'a>(slice: &'a mut [T], size: Dim<'n>) -> &'a mut Self {
        assert!(slice.len() == size.unbound());
        unsafe { &mut *(slice as *mut [T] as *mut Self) }
    }

    /// Returns the unconstrained slice.
    #[inline]
    #[track_caller]
    pub fn as_ref(&self) -> &[T] {
        unsafe { &*(self as *const _ as *const _) }
    }

    /// Returns the unconstrained slice.
    #[inline]
    #[track_caller]
    pub fn as_mut<'a>(&mut self) -> &'a mut [T] {
        unsafe { &mut *(self as *mut _ as *mut _) }
    }

    /// Returns the length of `self`.
    #[inline]
    pub fn len(&self) -> Dim<'n> {
        unsafe { Dim::new_unbound(self.unbound.len()) }
    }
}

impl<T: core::fmt::Debug> core::fmt::Debug for Array<'_, T> {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.unbound.fmt(f)
    }
}

impl<'n, T> core::ops::Index<Range<IdxInc<'n>>> for Array<'n, T> {
    type Output = [T];
    #[track_caller]
    fn index(&self, idx: Range<IdxInc<'n>>) -> &Self::Output {
        #[cfg(debug_assertions)]
        {
            &self.unbound[idx.start.unbound()..idx.end.unbound()]
        }
        #[cfg(not(debug_assertions))]
        unsafe {
            self.unbound
                .get_unchecked(idx.start.unbound()..idx.end.unbound())
        }
    }
}
impl<'n, T> core::ops::IndexMut<Range<IdxInc<'n>>> for Array<'n, T> {
    #[track_caller]
    fn index_mut(&mut self, idx: Range<IdxInc<'n>>) -> &mut Self::Output {
        #[cfg(debug_assertions)]
        {
            &mut self.unbound[idx.start.unbound()..idx.end.unbound()]
        }
        #[cfg(not(debug_assertions))]
        unsafe {
            self.unbound
                .get_unchecked_mut(idx.start.unbound()..idx.end.unbound())
        }
    }
}
impl<'n, T> core::ops::Index<Idx<'n>> for Array<'n, T> {
    type Output = T;
    #[track_caller]
    fn index(&self, idx: Idx<'n>) -> &Self::Output {
        #[cfg(debug_assertions)]
        {
            &self.unbound[idx.unbound()]
        }
        #[cfg(not(debug_assertions))]
        unsafe {
            self.unbound.get_unchecked(idx.unbound())
        }
    }
}
impl<'n, T> core::ops::IndexMut<Idx<'n>> for Array<'n, T> {
    #[track_caller]
    fn index_mut(&mut self, idx: Idx<'n>) -> &mut Self::Output {
        #[cfg(debug_assertions)]
        {
            &mut self.unbound[idx.unbound()]
        }
        #[cfg(not(debug_assertions))]
        unsafe {
            self.unbound.get_unchecked_mut(idx.unbound())
        }
    }
}
