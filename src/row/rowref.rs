use super::*;
use crate::{
    assert,
    col::ColRef,
    debug_assert,
    iter::{self, chunks::ChunkPolicy},
    Idx, IdxInc, Shape, Unbind,
};

/// Immutable view over a row vector, similar to an immutable reference to a strided [prim@slice].
///
/// # Note
///
/// Unlike a slice, the data pointed to by `RowRef<'_, E>` is allowed to be partially or fully
/// uninitialized under certain conditions. In this case, care must be taken to not perform any
/// operations that read the uninitialized values, or form references to them, either directly
/// through [`RowRef::read`], or indirectly through any of the numerical library routines, unless
/// it is explicitly permitted.
#[repr(C)]
pub struct RowRef<'a, E: Entity, C: Shape = usize> {
    pub(super) inner: VecImpl<E, C>,
    pub(super) __marker: PhantomData<&'a E>,
}

impl<E: Entity, C: Shape> Clone for RowRef<'_, E, C> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<E: Entity, C: Shape> Copy for RowRef<'_, E, C> {}

impl<E: Entity> Default for RowRef<'_, E> {
    #[inline]
    fn default() -> Self {
        from_slice_generic::<E>(E::faer_map(E::UNIT, |()| &[] as &[E::Unit]))
    }
}

impl<'short, E: Entity, C: Shape> Reborrow<'short> for RowRef<'_, E, C> {
    type Target = RowRef<'short, E, C>;

    #[inline]
    fn rb(&'short self) -> Self::Target {
        *self
    }
}

impl<'short, E: Entity, C: Shape> ReborrowMut<'short> for RowRef<'_, E, C> {
    type Target = RowRef<'short, E, C>;

    #[inline]
    fn rb_mut(&'short mut self) -> Self::Target {
        *self
    }
}

impl<E: Entity, C: Shape> IntoConst for RowRef<'_, E, C> {
    type Target = Self;

    #[inline]
    fn into_const(self) -> Self::Target {
        self
    }
}

impl<'a, E: Entity, C: Shape> RowRef<'a, E, C> {
    pub(crate) unsafe fn __from_raw_parts(ptr: PtrConst<E>, ncols: C, col_stride: isize) -> Self {
        Self {
            inner: VecImpl {
                ptr: into_copy::<E, _>(E::faer_map(
                    ptr,
                    #[inline]
                    |ptr| NonNull::new_unchecked(ptr as *mut E::Unit),
                )),
                len: ncols,
                stride: col_stride,
            },
            __marker: PhantomData,
        }
    }

    /// Returns the number of rows of the row. This is always equal to `1`.
    #[inline(always)]
    pub fn nrows(&self) -> usize {
        1
    }
    /// Returns the number of columns of the row.
    #[inline(always)]
    pub fn ncols(&self) -> C {
        self.inner.len
    }

    /// Returns pointers to the matrix data.
    #[inline(always)]
    pub fn as_ptr(self) -> PtrConst<E> {
        E::faer_map(
            from_copy::<E, _>(self.inner.ptr),
            #[inline(always)]
            |ptr| ptr.as_ptr() as *const E::Unit,
        )
    }

    /// Returns the column stride of the matrix, specified in number of elements, not in bytes.
    #[inline(always)]
    pub fn col_stride(&self) -> isize {
        self.inner.stride
    }

    /// Returns `self` as a matrix view.
    #[inline(always)]
    pub fn as_2d(self) -> MatRef<'a, E, usize, C> {
        let ncols = self.ncols();
        let col_stride = self.col_stride();
        unsafe { crate::mat::from_raw_parts(self.as_ptr(), 1, ncols, isize::MAX, col_stride) }
    }

    /// Returns raw pointers to the element at the given index.
    #[inline(always)]
    pub fn ptr_at(self, col: usize) -> PtrConst<E> {
        let offset = (col as isize).wrapping_mul(self.inner.stride);

        E::faer_map(
            self.as_ptr(),
            #[inline(always)]
            |ptr| ptr.wrapping_offset(offset),
        )
    }

    #[inline(always)]
    #[doc(hidden)]
    pub unsafe fn ptr_at_unchecked(self, col: usize) -> PtrConst<E> {
        let offset = crate::utils::unchecked_mul(col, self.inner.stride);
        E::faer_map(
            self.as_ptr(),
            #[inline(always)]
            |ptr| ptr.offset(offset),
        )
    }

    #[inline(always)]
    #[doc(hidden)]
    pub unsafe fn overflowing_ptr_at(self, col: IdxInc<C>) -> PtrConst<E> {
        unsafe {
            let cond = col != self.ncols();
            let offset = (cond as usize).wrapping_neg() as isize
                & (col.unbound() as isize).wrapping_mul(self.inner.stride);
            E::faer_map(
                self.as_ptr(),
                #[inline(always)]
                |ptr| ptr.offset(offset),
            )
        }
    }

    /// Returns raw pointers to the element at the given index, assuming the provided index
    /// is within the size of the vector.
    ///
    /// # Safety
    /// The behavior is undefined if any of the following conditions are violated:
    /// * `col < self.ncols()`.
    #[inline(always)]
    #[track_caller]
    pub unsafe fn ptr_inbounds_at(self, col: Idx<C>) -> PtrConst<E> {
        debug_assert!(col < self.ncols());
        self.ptr_at_unchecked(col.unbound())
    }

    /// Returns the input row with dynamic shape.
    #[inline]
    pub fn as_dyn(self) -> RowRef<'a, E> {
        let ncols = self.ncols().unbound();
        let col_stride = self.col_stride();
        unsafe { from_raw_parts(self.as_ptr(), ncols, col_stride) }
    }

    /// Returns the input row with the given shape after checking that it matches the
    /// current shape.
    #[inline]
    pub fn as_shape<H: Shape>(self, ncols: H) -> RowRef<'a, E, H> {
        assert!(ncols.unbound() == self.ncols().unbound());
        unsafe { from_raw_parts(self.as_ptr(), ncols, self.col_stride()) }
    }

    #[doc(hidden)]
    #[inline(always)]
    pub unsafe fn const_cast(self) -> RowMut<'a, E, C> {
        RowMut {
            inner: self.inner,
            __marker: PhantomData,
        }
    }

    /// Splits the column vector at the given index into two parts and
    /// returns an array of each subvector, in the following order:
    /// * left.
    /// * right.
    ///
    /// # Safety
    /// The behavior is undefined if any of the following conditions are violated:
    /// * `col <= self.ncols()`.
    #[inline(always)]
    #[track_caller]
    pub unsafe fn split_at_unchecked(
        self,
        col: IdxInc<C>,
    ) -> (RowRef<'a, E, usize>, RowRef<'a, E, usize>) {
        debug_assert!(col <= self.ncols());
        let col_stride = self.col_stride();
        let ncols = self.ncols().unbound();

        unsafe {
            let top = self.as_ptr();
            let bot = self.overflowing_ptr_at(col);
            let col = col.unbound();

            (
                RowRef::__from_raw_parts(top, col, col_stride),
                RowRef::__from_raw_parts(bot, ncols - col, col_stride),
            )
        }
    }

    /// Splits the column vector at the given index into two parts and
    /// returns an array of each subvector, in the following order, C:
    /// * top.
    /// * bottom.
    ///
    /// # Panics
    /// The function panics if any of the following conditions are violated:
    /// * `col <= self.ncols()`.
    #[inline(always)]
    #[track_caller]
    pub fn split_at(self, col: IdxInc<C>) -> (RowRef<'a, E, usize>, RowRef<'a, E, usize>) {
        assert!(col <= self.ncols());
        unsafe { self.split_at_unchecked(col) }
    }

    /// Returns references to the element at the given index, or subvector if `row` is a
    /// range.
    ///
    /// # Note
    /// The values pointed to by the references are expected to be initialized, even if the
    /// pointed-to value is not read, otherwise the behavior is undefined.
    ///
    /// # Safety
    /// The behavior is undefined if any of the following conditions are violated:
    /// * `col` must be contained in `[0, self.nc, Cols())`.
    #[inline(always)]
    #[track_caller]
    pub unsafe fn get_unchecked<ColRange>(
        self,
        col: ColRange,
    ) -> <Self as RowIndex<ColRange>>::Target
    where
        Self: RowIndex<ColRange>,
    {
        <Self as RowIndex<ColRange>>::get_unchecked(self, col)
    }

    /// Returns references to the element at the given index, or subvector if `col` is a
    /// range, with bound checks.
    ///
    /// # Note
    /// The values pointed to by the references are expected to be initialized, even if the
    /// pointed-to value is not read, otherwise the behavior is undefined.
    ///
    /// # Panics
    /// The function panics if any of the following conditions are violated:
    /// * `col` must be contained in `[0, self.ncols())`.
    #[inline(always)]
    #[track_caller]
    pub fn get<ColRange>(self, col: ColRange) -> <Self as RowIndex<ColRange>>::Target
    where
        Self: RowIndex<ColRange>,
    {
        <Self as RowIndex<ColRange>>::get(self, col)
    }

    /// Returns references to the element at the given index, or subvector if `row` is a
    /// range.
    ///
    /// # Note
    /// The values pointed to by the references are expected to be initialized, even if the
    /// pointed-to value is not read, otherwise the behavior is undefined.
    ///
    /// # Safety
    /// The behavior is undefined if any of the following conditions are violated:
    /// * `col` must be contained in `[0, self.nc, Cols())`.
    #[inline(always)]
    #[track_caller]
    pub unsafe fn at_unchecked(self, col: Idx<C>) -> Ref<'a, E> {
        self.transpose().at_unchecked(col)
    }

    /// Returns references to the element at the given index, or subvector if `col` is a
    /// range, with bound checks.
    ///
    /// # Note
    /// The values pointed to by the references are expected to be initialized, even if the
    /// pointed-to value is not read, otherwise the behavior is undefined.
    ///
    /// # Panics
    /// The function panics if any of the following conditions are violated:
    /// * `col` must be contained in `[0, self.ncols())`.
    #[inline(always)]
    #[track_caller]
    pub fn at(self, col: Idx<C>) -> Ref<'a, E> {
        self.transpose().at(col)
    }

    /// Reads the value of the element at the given index.
    ///
    /// # Safety
    /// The behavior is undefined if any of the following conditions are violated:
    /// * `col < self.ncols()`.
    #[inline(always)]
    #[track_caller]
    pub unsafe fn read_unchecked(&self, col: Idx<C>) -> E {
        E::faer_from_units(E::faer_map(
            self.at_unchecked(col),
            #[inline(always)]
            |ptr| *ptr,
        ))
    }

    /// Reads the value of the element at the given index, with bound checks.
    ///
    /// # Panics
    /// The function panics if any of the following conditions are violated:
    /// * `col < self.ncols()`.
    #[inline(always)]
    #[track_caller]
    pub fn read(&self, col: Idx<C>) -> E {
        E::faer_from_units(E::faer_map(
            self.at(col),
            #[inline(always)]
            |ptr| *ptr,
        ))
    }

    /// Returns a view over the transpose of `self`.
    #[inline(always)]
    #[must_use]
    pub fn transpose(self) -> ColRef<'a, E, C> {
        unsafe { ColRef::__from_raw_parts(self.as_ptr(), self.ncols(), self.col_stride()) }
    }

    /// Returns a view over the conjugate of `self`.
    #[inline(always)]
    #[must_use]
    pub fn conjugate(self) -> RowRef<'a, E::Conj, C>
    where
        E: Conjugate,
    {
        unsafe {
            // SAFETY: Conjugate requires that E::Unit and E::Conj::Unit have the same layout
            // and that GroupCopyFor<E,X> == E::Conj::GroupCopy<X>
            super::from_raw_parts(
                transmute_unchecked::<
                    GroupFor<E, *const UnitFor<E>>,
                    GroupFor<E::Conj, *const UnitFor<E::Conj>>,
                >(self.as_ptr()),
                self.ncols(),
                self.col_stride(),
            )
        }
    }

    /// Returns a view over the conjugate transpose of `self`.
    #[inline(always)]
    pub fn adjoint(self) -> ColRef<'a, E::Conj, C>
    where
        E: Conjugate,
    {
        self.conjugate().transpose()
    }

    /// Returns a view over the canonical representation of `self`, as well as a flag declaring
    /// whether `self` is implicitly conjugated or not.
    #[inline(always)]
    pub fn canonicalize(self) -> (RowRef<'a, E::Canonical, C>, Conj)
    where
        E: Conjugate,
    {
        (
            unsafe {
                // SAFETY: see Self::conjugate
                super::from_raw_parts(
                    transmute_unchecked::<
                        PtrConst<E>,
                        GroupFor<E::Canonical, *const UnitFor<E::Canonical>>,
                    >(self.as_ptr()),
                    self.ncols(),
                    self.col_stride(),
                )
            },
            if coe::is_same::<E, E::Canonical>() {
                Conj::No
            } else {
                Conj::Yes
            },
        )
    }

    /// Returns a view over the `self`, with the columns in reversed order.
    #[inline(always)]
    #[must_use]
    pub fn reverse_cols(self) -> Self {
        let ncols = self.ncols();
        let col_stride = self.col_stride().wrapping_neg();

        let ptr = unsafe { self.ptr_at_unchecked(ncols.unbound().saturating_sub(1)) };
        unsafe { Self::__from_raw_parts(ptr, ncols, col_stride) }
    }

    /// Returns a view over the subvector starting at column `col_start`, and with number of
    /// columns `ncols`.
    ///
    /// # Safety
    /// The behavior is undefined if any of the following conditions are violated:
    /// * `col_start <= self.ncols()`.
    /// * `ncols <= self.ncols() - col_start`.
    #[track_caller]
    #[inline(always)]
    pub unsafe fn subcols_unchecked<H: Shape>(
        self,
        col_start: IdxInc<C>,
        ncols: H,
    ) -> RowRef<'a, E, H> {
        debug_assert!(col_start <= self.ncols());
        {
            let ncols = ncols.unbound();
            let col_start = col_start.unbound();
            debug_assert!(ncols <= self.ncols().unbound() - col_start);
        }
        let col_stride = self.col_stride();
        unsafe { RowRef::__from_raw_parts(self.overflowing_ptr_at(col_start), ncols, col_stride) }
    }

    /// Returns a view over the subvector starting at col `col_start`, and with number of cols
    /// `ncols`.
    ///
    /// # Panics
    /// The function panics if any of the following conditions are violated:
    /// * `col_start <= self.ncols()`.
    /// * `ncols <= self.ncols() - col_start`.
    #[track_caller]
    #[inline(always)]
    pub fn subcols<H: Shape>(self, col_start: IdxInc<C>, ncols: H) -> RowRef<'a, E, H> {
        assert!(col_start <= self.ncols());
        {
            let ncols = ncols.unbound();
            let col_start = col_start.unbound();
            assert!(ncols <= self.ncols().unbound() - col_start);
        }
        unsafe { self.subcols_unchecked(col_start, ncols) }
    }

    /// Returns an owning [`Row`] of the data.
    #[inline]
    pub fn to_owned(&self) -> Row<E::Canonical, C>
    where
        E: Conjugate,
    {
        Row::from_fn(
            self.ncols(),
            #[inline(always)]
            |i| unsafe { self.read_unchecked(i) }.canonicalize(),
        )
    }

    /// Returns `true` if any of the elements is NaN, otherwise returns `false`.
    #[inline]
    pub fn has_nan(&self) -> bool
    where
        E: ComplexField,
    {
        (*self).rb().as_2d().has_nan()
    }

    /// Returns `true` if all of the elements are finite, otherwise returns `false`.
    #[inline]
    pub fn is_all_finite(&self) -> bool
    where
        E: ComplexField,
    {
        (*self).rb().as_2d().is_all_finite()
    }

    /// Returns the maximum norm of `self`.
    #[inline]
    pub fn norm_max(&self) -> E::Real
    where
        E: ComplexField,
    {
        self.as_2d().norm_max()
    }
    /// Returns the L1 norm of `self`.
    #[inline]
    pub fn norm_l1(&self) -> E::Real
    where
        E: ComplexField,
    {
        self.as_ref().as_2d().norm_l1()
    }

    /// Returns the L2 norm of `self`.
    #[inline]
    pub fn norm_l2(&self) -> E::Real
    where
        E: ComplexField,
    {
        self.as_ref().as_2d().norm_l2()
    }

    /// Returns the squared L2 norm of `self`.
    #[inline]
    pub fn squared_norm_l2(&self) -> E::Real
    where
        E: ComplexField,
    {
        self.as_ref().as_2d().squared_norm_l2()
    }

    /// Returns the sum of `self`.
    #[inline]
    pub fn sum(&self) -> E
    where
        E: ComplexField,
    {
        self.as_2d().sum()
    }

    /// Kronecker product of `self` and `rhs`.
    ///
    /// This is an allocating operation; see [`faer::linalg::kron`](crate::linalg::kron) for the
    /// allocation-free version or more info in general.
    #[inline]
    #[track_caller]
    pub fn kron(&self, rhs: impl As2D<E>) -> Mat<E>
    where
        E: ComplexField,
    {
        self.as_2d().kron(rhs)
    }

    /// Returns the row as a contiguous slice if its column stride is equal to `1`.
    ///
    /// # Note
    /// The values pointed to by the references are expected to be initialized, even if the
    /// pointed-to value is not read, otherwise the behavior is undefined.
    #[inline]
    pub fn try_as_slice(self) -> Option<Slice<'a, E>> {
        if self.col_stride() == 1 {
            let len = self.ncols().unbound();
            Some(E::faer_map(
                self.as_ptr(),
                #[inline(always)]
                |ptr| unsafe { core::slice::from_raw_parts(ptr, len) },
            ))
        } else {
            None
        }
    }

    /// Returns a view over the matrix.
    #[inline]
    pub fn as_ref(&self) -> RowRef<'_, E, C> {
        *self
    }

    /// Returns a reference to the first element and a view over the remaining ones if the row is
    /// non-empty, otherwise `None`.
    #[inline]
    pub fn split_first(self) -> Option<(Ref<'a, E>, RowRef<'a, E>)> {
        let this = self.as_dyn();
        if this.ncols() == 0 {
            None
        } else {
            unsafe {
                let (head, tail) = { this.split_at_unchecked(1) };
                Some((head.get_unchecked(0), tail))
            }
        }
    }

    /// Returns a reference to the last element and a view over the remaining ones if the row is
    /// non-empty, otherwise `None`.
    #[inline]
    pub fn split_last(self) -> Option<(Ref<'a, E>, RowRef<'a, E>)> {
        let this = self.as_dyn();
        if this.ncols() == 0 {
            None
        } else {
            unsafe {
                let (head, tail) = { this.split_at_unchecked(this.ncols() - 1) };
                Some((tail.get_unchecked(0), head))
            }
        }
    }

    /// Returns an iterator over the elements of the row.
    #[inline]
    pub fn iter(self) -> iter::ElemIter<'a, E> {
        iter::ElemIter {
            inner: self.transpose().as_dyn(),
        }
    }

    /// Returns an iterator that provides successive chunks of the elements of this row, with
    /// each having at most `chunk_size` elements.
    #[inline]
    #[track_caller]
    pub fn chunks(self, chunk_size: usize) -> iter::RowElemChunks<'a, E> {
        assert!(chunk_size > 0);
        iter::RowElemChunks {
            inner: self.as_dyn(),
            policy: iter::chunks::ChunkSizePolicy::new(
                self.ncols().unbound(),
                iter::chunks::ChunkSize(chunk_size),
            ),
        }
    }

    /// Returns an iterator that provides exactly `count` successive chunks of the elements of this
    /// row.
    #[inline]
    #[track_caller]
    pub fn partition(self, count: usize) -> iter::RowElemPartition<'a, E> {
        assert!(count > 0);
        iter::RowElemPartition {
            inner: self.as_dyn(),
            policy: iter::chunks::PartitionCountPolicy::new(
                self.ncols().unbound(),
                iter::chunks::PartitionCount(count),
            ),
        }
    }

    /// Returns an iterator that provides successive chunks of the elements of this row, with
    /// each having at most `chunk_size` elements.
    ///
    /// Only available with the `rayon` feature.
    #[cfg(feature = "rayon")]
    #[cfg_attr(docsrs, doc(cfg(feature = "rayon")))]
    #[inline]
    #[track_caller]
    pub fn par_chunks(
        self,
        chunk_size: usize,
    ) -> impl 'a + rayon::iter::IndexedParallelIterator<Item = RowRef<'a, E>> {
        use rayon::prelude::*;

        self.transpose()
            .par_chunks(chunk_size)
            .map(|x| x.transpose())
    }

    /// Returns an iterator that provides exactly `count` successive chunks of the elements of this
    /// row.
    ///
    /// Only available with the `rayon` feature.
    #[cfg(feature = "rayon")]
    #[cfg_attr(docsrs, doc(cfg(feature = "rayon")))]
    #[inline]
    #[track_caller]
    pub fn par_partition(
        self,
        count: usize,
    ) -> impl 'a + rayon::iter::IndexedParallelIterator<Item = RowRef<'a, E>> {
        use rayon::prelude::*;

        self.transpose().par_partition(count).map(|x| x.transpose())
    }
}

/// Creates a `RowRef` from pointers to the row vector data, number of columns, and column
/// stride.
///
/// # Safety:
/// This function has the same safety requirements as
/// [`mat::from_raw_parts(ptr, 1, ncols, 0, col_stride)`]
#[inline(always)]
pub unsafe fn from_raw_parts<'a, E: Entity, C: Shape>(
    ptr: PtrConst<E>,
    ncols: C,
    col_stride: isize,
) -> RowRef<'a, E, C> {
    RowRef::__from_raw_parts(ptr, ncols, col_stride)
}

/// Creates a `RowRef` from slice views over the row vector data, The result has the same
/// number of columns as the length of the input slice.
#[inline(always)]
pub fn from_slice_generic<E: Entity>(slice: Slice<'_, E>) -> RowRef<'_, E> {
    let nrows = SliceGroup::<'_, E>::new(E::faer_copy(&slice)).len();

    unsafe {
        from_raw_parts(
            E::faer_map(
                slice,
                #[inline(always)]
                |slice| slice.as_ptr(),
            ),
            nrows,
            1,
        )
    }
}

/// Creates a `RowRef` from slice views over the row vector data, The result has the same
/// number of columns as the length of the input slice.
#[inline(always)]
pub fn from_slice<E: SimpleEntity>(slice: &[E]) -> RowRef<'_, E> {
    from_slice_generic(slice)
}

impl<E: Entity, C: Shape> As2D<E> for RowRef<'_, E, C> {
    #[inline]
    fn as_2d_ref(&self) -> MatRef<'_, E> {
        (*self).as_2d().as_dyn()
    }
}

impl<E: Entity, C: Shape> AsRowRef<E> for RowRef<'_, E, C> {
    type C = C;

    #[inline]
    fn as_row_ref(&self) -> RowRef<'_, E, C> {
        *self
    }
}

impl<'a, E: Entity, C: Shape> core::fmt::Debug for RowRef<'a, E, C> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.transpose().fmt(f)
    }
}

impl<E: SimpleEntity, C: Shape> core::ops::Index<Idx<C>> for RowRef<'_, E, C> {
    type Output = E;

    #[inline]
    #[track_caller]
    fn index(&self, col: Idx<C>) -> &E {
        self.at(col)
    }
}

impl<E: Conjugate> RowBatch<E> for RowRef<'_, E> {
    type Owned = Row<E::Canonical>;

    #[inline]
    #[track_caller]
    fn new_owned_zeros(nrows: usize, ncols: usize) -> Self::Owned {
        assert!(nrows == 1);
        Row::zeros(ncols)
    }

    #[inline]
    fn new_owned_copied(src: &Self) -> Self::Owned {
        src.to_owned()
    }

    #[inline]
    fn resize_owned(owned: &mut Self::Owned, nrows: usize, ncols: usize) {
        <Self::Owned as RowBatch<E::Canonical>>::resize_owned(owned, nrows, ncols)
    }
}

/// Returns a view over a row with `ncols` columns containing `value` repeated for all elements.
#[doc(alias = "broadcast")]
pub fn from_repeated_ref<E: SimpleEntity>(value: &E, ncols: usize) -> RowRef<'_, E> {
    unsafe { from_raw_parts(E::faer_map(value, |ptr| ptr as *const E::Unit), ncols, 0) }
}

/// Returns a view over a row with 1 column containing value as its only element, pointing to
/// `value`.
pub fn from_ref<E: SimpleEntity>(value: &E) -> RowRef<'_, E> {
    from_ref_generic(value)
}

/// Returns a view over a row with `ncols` columns containing `value` repeated for all elements.
#[doc(alias = "broadcast")]
pub fn from_repeated_ref_generic<E: Entity>(value: Ref<'_, E>, ncols: usize) -> RowRef<'_, E> {
    unsafe { from_raw_parts(E::faer_map(value, |ptr| ptr as *const E::Unit), ncols, 0) }
}

/// Returns a view over a row with 1 column containing value as its only element, pointing to
/// `value`.
pub fn from_ref_generic<E: Entity>(value: Ref<'_, E>) -> RowRef<'_, E> {
    from_repeated_ref_generic(value, 1)
}
