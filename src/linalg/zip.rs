//! Implementation of [`zipped_rw!`] structures.

use crate::{assert, debug_assert, *};
use core::mem::MaybeUninit;
use faer_entity::*;
use reborrow::*;

#[doc(hidden)]
pub struct RefWrapper<M>(pub M);

/// Read only view over a single matrix element.
pub struct Read<'a, E: Entity> {
    ptr: GroupFor<E, &'a MaybeUninit<E::Unit>>,
}
/// Read-write view over a single matrix element.
pub struct ReadWrite<'a, E: Entity> {
    ptr: GroupFor<E, &'a mut MaybeUninit<E::Unit>>,
}

/// Type that can be converted to a view.
pub trait ViewMut {
    /// View type.
    type Target<'a>
    where
        Self: 'a;

    /// Returns the view over `this`.
    fn view_mut(this: &mut Self) -> Self::Target<'_>;
}

impl<E: Entity> ViewMut for Row<E> {
    type Target<'a>
        = RowRef<'a, E>
    where
        Self: 'a;

    #[inline]
    fn view_mut(this: &mut Self) -> Self::Target<'_> {
        this.as_ref()
    }
}
impl<E: Entity> ViewMut for &Row<E> {
    type Target<'a>
        = RowRef<'a, E>
    where
        Self: 'a;

    #[inline]
    fn view_mut(this: &mut Self) -> Self::Target<'_> {
        (*this).as_ref()
    }
}
impl<E: Entity> ViewMut for &mut Row<E> {
    type Target<'a>
        = RowMut<'a, E>
    where
        Self: 'a;

    #[inline]
    fn view_mut(this: &mut Self) -> Self::Target<'_> {
        (*this).as_mut()
    }
}

impl<E: Entity, C: Shape> ViewMut for RowRef<'_, E, C> {
    type Target<'a>
        = RowRef<'a, E, C>
    where
        Self: 'a;

    #[inline]
    fn view_mut(this: &mut Self) -> Self::Target<'_> {
        *this
    }
}
impl<E: Entity, C: Shape> ViewMut for RowMut<'_, E, C> {
    type Target<'a>
        = RowMut<'a, E, C>
    where
        Self: 'a;

    #[inline]
    fn view_mut(this: &mut Self) -> Self::Target<'_> {
        (*this).rb_mut()
    }
}
impl<E: Entity, C: Shape> ViewMut for &mut RowRef<'_, E, C> {
    type Target<'a>
        = RowRef<'a, E, C>
    where
        Self: 'a;

    #[inline]
    fn view_mut(this: &mut Self) -> Self::Target<'_> {
        **this
    }
}
impl<E: Entity, C: Shape> ViewMut for &mut RowMut<'_, E, C> {
    type Target<'a>
        = RowMut<'a, E, C>
    where
        Self: 'a;

    #[inline]
    fn view_mut(this: &mut Self) -> Self::Target<'_> {
        (**this).rb_mut()
    }
}
impl<E: Entity, C: Shape> ViewMut for &RowRef<'_, E, C> {
    type Target<'a>
        = RowRef<'a, E, C>
    where
        Self: 'a;

    #[inline]
    fn view_mut(this: &mut Self) -> Self::Target<'_> {
        **this
    }
}
impl<E: Entity, C: Shape> ViewMut for &RowMut<'_, E, C> {
    type Target<'a>
        = RowRef<'a, E, C>
    where
        Self: 'a;

    #[inline]
    fn view_mut(this: &mut Self) -> Self::Target<'_> {
        (**this).rb()
    }
}

impl<E: Entity, R: Shape> ViewMut for Col<E, R> {
    type Target<'a>
        = ColRef<'a, E, R>
    where
        Self: 'a;

    #[inline]
    fn view_mut(this: &mut Self) -> Self::Target<'_> {
        this.as_ref()
    }
}
impl<E: Entity, R: Shape> ViewMut for &Col<E, R> {
    type Target<'a>
        = ColRef<'a, E, R>
    where
        Self: 'a;

    #[inline]
    fn view_mut(this: &mut Self) -> Self::Target<'_> {
        (*this).as_ref()
    }
}
impl<E: Entity, R: Shape> ViewMut for &mut Col<E, R> {
    type Target<'a>
        = ColMut<'a, E, R>
    where
        Self: 'a;

    #[inline]
    fn view_mut(this: &mut Self) -> Self::Target<'_> {
        (*this).as_mut()
    }
}

impl<E: Entity, R: Shape> ViewMut for ColRef<'_, E, R> {
    type Target<'a>
        = ColRef<'a, E, R>
    where
        Self: 'a;

    #[inline]
    fn view_mut(this: &mut Self) -> Self::Target<'_> {
        *this
    }
}
impl<E: Entity, R: Shape> ViewMut for ColMut<'_, E, R> {
    type Target<'a>
        = ColMut<'a, E, R>
    where
        Self: 'a;

    #[inline]
    fn view_mut(this: &mut Self) -> Self::Target<'_> {
        (*this).rb_mut()
    }
}
impl<E: Entity, R: Shape> ViewMut for &mut ColRef<'_, E, R> {
    type Target<'a>
        = ColRef<'a, E, R>
    where
        Self: 'a;

    #[inline]
    fn view_mut(this: &mut Self) -> Self::Target<'_> {
        **this
    }
}
impl<E: Entity, R: Shape> ViewMut for &mut ColMut<'_, E, R> {
    type Target<'a>
        = ColMut<'a, E, R>
    where
        Self: 'a;

    #[inline]
    fn view_mut(this: &mut Self) -> Self::Target<'_> {
        (**this).rb_mut()
    }
}
impl<E: Entity, R: Shape> ViewMut for &ColRef<'_, E, R> {
    type Target<'a>
        = ColRef<'a, E, R>
    where
        Self: 'a;

    #[inline]
    fn view_mut(this: &mut Self) -> Self::Target<'_> {
        **this
    }
}
impl<E: Entity, R: Shape> ViewMut for &ColMut<'_, E, R> {
    type Target<'a>
        = ColRef<'a, E, R>
    where
        Self: 'a;

    #[inline]
    fn view_mut(this: &mut Self) -> Self::Target<'_> {
        (**this).rb()
    }
}

impl<E: Entity, R: Shape, C: Shape> ViewMut for Mat<E, R, C> {
    type Target<'a>
        = MatRef<'a, E, R, C>
    where
        Self: 'a;

    #[inline]
    fn view_mut(this: &mut Self) -> Self::Target<'_> {
        this.as_ref()
    }
}
impl<E: Entity, R: Shape, C: Shape> ViewMut for &Mat<E, R, C> {
    type Target<'a>
        = MatRef<'a, E, R, C>
    where
        Self: 'a;

    #[inline]
    fn view_mut(this: &mut Self) -> Self::Target<'_> {
        (*this).as_ref()
    }
}
impl<E: Entity, R: Shape, C: Shape> ViewMut for &mut Mat<E, R, C> {
    type Target<'a>
        = MatMut<'a, E, R, C>
    where
        Self: 'a;

    #[inline]
    fn view_mut(this: &mut Self) -> Self::Target<'_> {
        (*this).as_mut()
    }
}

impl<E: Entity, R: Shape, C: Shape> ViewMut for MatRef<'_, E, R, C> {
    type Target<'a>
        = MatRef<'a, E, R, C>
    where
        Self: 'a;

    #[inline]
    fn view_mut(this: &mut Self) -> Self::Target<'_> {
        *this
    }
}
impl<E: Entity, R: Shape, C: Shape> ViewMut for MatMut<'_, E, R, C> {
    type Target<'a>
        = MatMut<'a, E, R, C>
    where
        Self: 'a;

    #[inline]
    fn view_mut(this: &mut Self) -> Self::Target<'_> {
        (*this).rb_mut()
    }
}
impl<E: Entity, R: Shape, C: Shape> ViewMut for &mut MatRef<'_, E, R, C> {
    type Target<'a>
        = MatRef<'a, E, R, C>
    where
        Self: 'a;

    #[inline]
    fn view_mut(this: &mut Self) -> Self::Target<'_> {
        **this
    }
}
impl<E: Entity, R: Shape, C: Shape> ViewMut for &mut MatMut<'_, E, R, C> {
    type Target<'a>
        = MatMut<'a, E, R, C>
    where
        Self: 'a;

    #[inline]
    fn view_mut(this: &mut Self) -> Self::Target<'_> {
        (**this).rb_mut()
    }
}
impl<E: Entity, R: Shape, C: Shape> ViewMut for &MatRef<'_, E, R, C> {
    type Target<'a>
        = MatRef<'a, E, R, C>
    where
        Self: 'a;

    #[inline]
    fn view_mut(this: &mut Self) -> Self::Target<'_> {
        **this
    }
}
impl<E: Entity, R: Shape, C: Shape> ViewMut for &MatMut<'_, E, R, C> {
    type Target<'a>
        = MatRef<'a, E, R, C>
    where
        Self: 'a;

    #[inline]
    fn view_mut(this: &mut Self) -> Self::Target<'_> {
        (**this).rb()
    }
}

impl<E: SimpleEntity> core::ops::Deref for Read<'_, E> {
    type Target = E;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        unsafe { &*(self.ptr as *const _ as *const E::Unit) }
    }
}
impl<E: SimpleEntity> core::ops::Deref for ReadWrite<'_, E> {
    type Target = E;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        unsafe { &*(self.ptr as *const _ as *const E::Unit) }
    }
}
impl<E: SimpleEntity> core::ops::DerefMut for ReadWrite<'_, E> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *(self.ptr as *mut _ as *mut E::Unit) }
    }
}

impl<E: Entity> Read<'_, E> {
    /// Read the value of the element.
    #[inline(always)]
    pub fn read(&self) -> E {
        E::faer_from_units(E::faer_map(
            E::faer_as_ref(&self.ptr),
            #[inline(always)]
            |ptr| unsafe { ptr.assume_init_read() },
        ))
    }
}
impl<E: Entity> ReadWrite<'_, E> {
    /// Read the value of the element.
    #[inline(always)]
    pub fn read(&self) -> E {
        E::faer_from_units(E::faer_map(
            E::faer_as_ref(&self.ptr),
            #[inline(always)]
            |ptr| unsafe { *ptr.assume_init_ref() },
        ))
    }

    /// Write to the location of the element.
    #[inline(always)]
    pub fn write(&mut self, value: E) {
        let value = E::faer_into_units(value);
        E::faer_map(
            E::faer_zip(E::faer_as_mut(&mut self.ptr), value),
            #[inline(always)]
            |(ptr, value)| unsafe { *ptr.assume_init_mut() = value },
        );
    }
}

/// Specifies whether the main diagonal should be traversed, when iterating over a triangular
/// chunk of the matrix.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Diag {
    /// Do not include diagonal of matrix
    Skip,
    /// Include diagonal of matrix
    Include,
}

/// Matrix layout transformation. Used for zipping optimizations.
#[derive(Copy, Clone)]
pub enum MatLayoutTransform {
    /// Matrix is used as-is.
    None,
    /// Matrix rows are reversed.
    ReverseRows,
    /// Matrix is transposed.
    Transpose,
    /// Matrix is transposed, then rows are reversed.
    TransposeReverseRows,
}

/// Vector layout transformation. Used for zipping optimizations.
#[derive(Copy, Clone)]
pub enum VecLayoutTransform {
    /// Vector is used as-is.
    None,
    /// Vector is reversed.
    Reverse,
}

/// Type with a given matrix shape.
pub trait MatShape {
    /// Type of rows.
    type Rows: Copy + Eq + core::fmt::Debug;
    /// Type of columns.
    type Cols: Copy + Eq + core::fmt::Debug;
    /// Returns the number of rows.
    fn nrows(this: &Self) -> Self::Rows;
    /// Returns the number of columns.
    fn ncols(this: &Self) -> Self::Cols;
}

/// Zipped matrix views.
pub unsafe trait MaybeContiguous: MatShape {
    /// Indexing type.
    type Index: Copy;
    /// Contiguous slice type.
    type Slice;
    /// Layout transformation type.
    type LayoutTransform: Copy;
    /// Returns slice at index of length `n_elems`.
    unsafe fn get_slice_unchecked(this: &mut Self, idx: Self::Index, n_elems: usize)
        -> Self::Slice;
}

/// Zipped matrix views.
pub unsafe trait MatIndex: MaybeContiguous {
    /// Item produced by the zipped views.
    type Item;
    /// Reference item produced by the zipped views.
    type RefItem;

    /// Convert item to reference.
    unsafe fn to_ref(item: Self::Item) -> Self::RefItem;

    /// Matrix type with type erased dimensions.
    type Dyn: MatIndex;

    /// Converts a type erased index back to its original representation.
    unsafe fn from_dyn_idx(idx: <Self::Dyn as MaybeContiguous>::Index) -> Self::Index;

    /// Get the item at the given index, skipping bound checks.
    unsafe fn get_unchecked(this: &mut Self, index: Self::Index) -> Self::Item;
    /// Get the item at the given slice position, skipping bound checks.
    unsafe fn next_unchecked(slice: &mut Self::Slice) -> Self::Item;

    /// Checks if the zipped matrices are contiguous.
    fn is_contiguous(this: &Self) -> bool;
    /// Computes the preferred iteration layout of the matrices.
    fn preferred_layout(this: &Self) -> Self::LayoutTransform;
    /// Applies the layout transformation to the matrices.
    fn with_layout(this: Self, layout: Self::LayoutTransform) -> Self::Dyn;
}

/// Single element.
#[derive(Copy, Clone, Debug)]
pub struct Last<Mat>(pub Mat);

/// Zipped elements.
#[derive(Copy, Clone, Debug)]
pub struct Zip<Head, Tail>(pub Head, pub Tail);

/// Single matrix view.
#[derive(Copy, Clone, Debug)]
pub struct LastEq<Rows, Cols, Mat: MatShape<Rows = Rows, Cols = Cols>>(pub Mat);
/// Zipped matrix views.
#[derive(Copy, Clone, Debug)]
pub struct ZipEq<
    Rows,
    Cols,
    Head: MatShape<Rows = Rows, Cols = Cols>,
    Tail: MatShape<Rows = Rows, Cols = Cols>,
>(Head, Tail);

impl<
        Rows: Copy + Eq + core::fmt::Debug,
        Cols: Copy + Eq + core::fmt::Debug,
        Head: MatShape<Rows = Rows, Cols = Cols>,
        Tail: MatShape<Rows = Rows, Cols = Cols>,
    > ZipEq<Rows, Cols, Head, Tail>
{
    /// Creates a zipped matrix, after asserting that the dimensions match.
    #[inline(always)]
    #[track_caller]
    pub fn new(head: Head, tail: Tail) -> Self {
        assert!(all(
            Head::nrows(&head) == Tail::nrows(&tail),
            Head::ncols(&head) == Tail::ncols(&tail),
        ));
        Self(head, tail)
    }

    /// Creates a zipped matrix, assuming that the dimensions match.
    #[inline(always)]
    #[track_caller]
    pub fn new_unchecked(head: Head, tail: Tail) -> Self {
        debug_assert!(all(
            Head::nrows(&head) == Tail::nrows(&tail),
            Head::ncols(&head) == Tail::ncols(&tail),
        ));
        Self(head, tail)
    }
}

impl<
        Rows: Copy + Eq + core::fmt::Debug,
        Cols: Copy + Eq + core::fmt::Debug,
        Mat: MatShape<Rows = Rows, Cols = Cols>,
    > MatShape for LastEq<Rows, Cols, Mat>
{
    type Rows = Rows;
    type Cols = Cols;
    #[inline(always)]
    fn nrows(this: &Self) -> Self::Rows {
        Mat::nrows(&this.0)
    }
    #[inline(always)]
    fn ncols(this: &Self) -> Self::Cols {
        Mat::ncols(&this.0)
    }
}

impl<Mat: MatShape> MatShape for RefWrapper<Mat> {
    type Rows = Mat::Rows;
    type Cols = Mat::Cols;
    #[inline(always)]
    fn nrows(this: &Self) -> Self::Rows {
        Mat::nrows(&this.0)
    }
    #[inline(always)]
    fn ncols(this: &Self) -> Self::Cols {
        Mat::ncols(&this.0)
    }
}

impl<
        Rows: Copy + Eq + core::fmt::Debug,
        Cols: Copy + Eq + core::fmt::Debug,
        Head: MatShape<Rows = Rows, Cols = Cols>,
        Tail: MatShape<Rows = Rows, Cols = Cols>,
    > MatShape for ZipEq<Rows, Cols, Head, Tail>
{
    type Rows = Rows;
    type Cols = Cols;
    #[inline(always)]
    fn nrows(this: &Self) -> Self::Rows {
        Head::nrows(&this.0)
    }
    #[inline(always)]
    fn ncols(this: &Self) -> Self::Cols {
        Head::ncols(&this.0)
    }
}

impl<E: Entity, R: Shape> MatShape for ColRef<'_, E, R> {
    type Rows = R;
    type Cols = ();
    #[inline(always)]
    fn nrows(this: &Self) -> Self::Rows {
        (*this).nrows()
    }
    #[inline(always)]
    fn ncols(_: &Self) -> Self::Cols {}
}

impl<E: Entity, R: Shape> MatShape for ColMut<'_, E, R> {
    type Rows = R;
    type Cols = ();
    #[inline(always)]
    fn nrows(this: &Self) -> Self::Rows {
        (*this).nrows()
    }
    #[inline(always)]
    fn ncols(_: &Self) -> Self::Cols {}
}

impl<E: Entity, C: Shape> MatShape for RowRef<'_, E, C> {
    type Rows = ();
    type Cols = C;
    #[inline(always)]
    fn nrows(_: &Self) -> Self::Rows {}
    #[inline(always)]
    fn ncols(this: &Self) -> Self::Cols {
        (*this).ncols()
    }
}
impl<E: Entity, C: Shape> MatShape for RowMut<'_, E, C> {
    type Rows = ();
    type Cols = C;
    #[inline(always)]
    fn nrows(_: &Self) -> Self::Rows {}
    #[inline(always)]
    fn ncols(this: &Self) -> Self::Cols {
        (*this).ncols()
    }
}

impl<E: Entity, R: Shape, C: Shape> MatShape for MatRef<'_, E, R, C> {
    type Rows = R;
    type Cols = C;
    #[inline(always)]
    fn nrows(this: &Self) -> Self::Rows {
        (*this).nrows()
    }
    #[inline(always)]
    fn ncols(this: &Self) -> Self::Cols {
        (*this).ncols()
    }
}

impl<E: Entity, R: Shape, C: Shape> MatShape for MatMut<'_, E, R, C> {
    type Rows = R;
    type Cols = C;
    #[inline(always)]
    fn nrows(this: &Self) -> Self::Rows {
        (*this).nrows()
    }
    #[inline(always)]
    fn ncols(this: &Self) -> Self::Cols {
        (*this).ncols()
    }
}

unsafe impl<
        Rows: Copy + Eq + core::fmt::Debug,
        Cols: Copy + Eq + core::fmt::Debug,
        Mat: MaybeContiguous<Rows = Rows, Cols = Cols>,
    > MaybeContiguous for LastEq<Rows, Cols, Mat>
{
    type Index = Mat::Index;
    type Slice = Last<Mat::Slice>;
    type LayoutTransform = Mat::LayoutTransform;
    #[inline(always)]
    unsafe fn get_slice_unchecked(
        this: &mut Self,
        idx: Self::Index,
        n_elems: usize,
    ) -> Self::Slice {
        Last(Mat::get_slice_unchecked(&mut this.0, idx, n_elems))
    }
}

unsafe impl<Mat: MaybeContiguous> MaybeContiguous for RefWrapper<Mat> {
    type Index = Mat::Index;
    type Slice = Mat::Slice;
    type LayoutTransform = Mat::LayoutTransform;
    #[inline(always)]
    unsafe fn get_slice_unchecked(
        this: &mut Self,
        idx: Self::Index,
        n_elems: usize,
    ) -> Self::Slice {
        Mat::get_slice_unchecked(&mut this.0, idx, n_elems)
    }
}

unsafe impl<
        Rows: Copy + Eq + core::fmt::Debug,
        Cols: Copy + Eq + core::fmt::Debug,
        Mat: MatIndex<Rows = Rows, Cols = Cols>,
    > MatIndex for LastEq<Rows, Cols, Mat>
{
    type Item = Last<Mat::Item>;
    type RefItem = Last<Mat::RefItem>;
    type Dyn = LastEq<<Mat::Dyn as MatShape>::Rows, <Mat::Dyn as MatShape>::Cols, Mat::Dyn>;

    #[inline(always)]
    unsafe fn to_ref(item: Self::Item) -> Self::RefItem {
        Last(Mat::to_ref(item.0))
    }

    #[inline(always)]
    unsafe fn get_unchecked(this: &mut Self, index: Self::Index) -> Self::Item {
        Last(Mat::get_unchecked(&mut this.0, index))
    }

    #[inline(always)]
    unsafe fn from_dyn_idx(idx: <Self::Dyn as MaybeContiguous>::Index) -> Self::Index {
        Mat::from_dyn_idx(idx)
    }

    #[inline(always)]
    unsafe fn next_unchecked(slice: &mut Self::Slice) -> Self::Item {
        Last(Mat::next_unchecked(&mut slice.0))
    }

    #[inline(always)]
    fn is_contiguous(this: &Self) -> bool {
        Mat::is_contiguous(&this.0)
    }
    #[inline(always)]
    fn preferred_layout(this: &Self) -> Self::LayoutTransform {
        Mat::preferred_layout(&this.0)
    }
    #[inline(always)]
    fn with_layout(this: Self, layout: Self::LayoutTransform) -> Self::Dyn {
        LastEq(Mat::with_layout(this.0, layout))
    }
}

unsafe impl<Mat: MatIndex> MatIndex for RefWrapper<Mat> {
    type Item = Mat::RefItem;
    type RefItem = Mat::RefItem;
    type Dyn = RefWrapper<Mat::Dyn>;

    #[inline(always)]
    unsafe fn to_ref(item: Self::Item) -> Self::RefItem {
        item
    }

    #[inline(always)]
    unsafe fn get_unchecked(this: &mut Self, index: Self::Index) -> Self::Item {
        Mat::to_ref(Mat::get_unchecked(&mut this.0, index))
    }

    #[inline(always)]
    unsafe fn from_dyn_idx(idx: <Self::Dyn as MaybeContiguous>::Index) -> Self::Index {
        Mat::from_dyn_idx(idx)
    }

    #[inline(always)]
    unsafe fn next_unchecked(slice: &mut Self::Slice) -> Self::Item {
        Mat::to_ref(Mat::next_unchecked(slice))
    }

    #[inline(always)]
    fn is_contiguous(this: &Self) -> bool {
        Mat::is_contiguous(&this.0)
    }
    #[inline(always)]
    fn preferred_layout(this: &Self) -> Self::LayoutTransform {
        Mat::preferred_layout(&this.0)
    }
    #[inline(always)]
    fn with_layout(this: Self, layout: Self::LayoutTransform) -> Self::Dyn {
        RefWrapper(Mat::with_layout(this.0, layout))
    }
}

unsafe impl<
        Rows: Copy + Eq + core::fmt::Debug,
        Cols: Copy + Eq + core::fmt::Debug,
        Head: MaybeContiguous<Rows = Rows, Cols = Cols>,
        Tail: MaybeContiguous<
            Rows = Rows,
            Cols = Cols,
            Index = Head::Index,
            LayoutTransform = Head::LayoutTransform,
        >,
    > MaybeContiguous for ZipEq<Rows, Cols, Head, Tail>
{
    type Index = Head::Index;
    type Slice = Zip<Head::Slice, Tail::Slice>;
    type LayoutTransform = Head::LayoutTransform;
    #[inline(always)]
    unsafe fn get_slice_unchecked(
        this: &mut Self,
        idx: Self::Index,
        n_elems: usize,
    ) -> Self::Slice {
        Zip(
            Head::get_slice_unchecked(&mut this.0, idx, n_elems),
            Tail::get_slice_unchecked(&mut this.1, idx, n_elems),
        )
    }
}

unsafe impl<
        Rows: Copy + Eq + core::fmt::Debug,
        Cols: Copy + Eq + core::fmt::Debug,
        Head: MatIndex<Rows = Rows, Cols = Cols, Dyn: MatIndex<Dyn = Head::Dyn>>,
        Tail: MatIndex<
            Rows = Rows,
            Cols = Cols,
            Dyn: MatIndex<
                Rows = <Head::Dyn as MatShape>::Rows,
                Cols = <Head::Dyn as MatShape>::Cols,
                Index = <Head::Dyn as MaybeContiguous>::Index,
                LayoutTransform = <Head::Dyn as MaybeContiguous>::LayoutTransform,
                Dyn = Tail::Dyn,
            >,
            Index = Head::Index,
            LayoutTransform = Head::LayoutTransform,
        >,
    > MatIndex for ZipEq<Rows, Cols, Head, Tail>
{
    type Item = Zip<Head::Item, Tail::Item>;
    type RefItem = Zip<Head::RefItem, Tail::RefItem>;

    #[inline(always)]
    unsafe fn to_ref(item: Self::Item) -> Self::RefItem {
        Zip(Head::to_ref(item.0), Tail::to_ref(item.1))
    }

    type Dyn =
        ZipEq<<Head::Dyn as MatShape>::Rows, <Head::Dyn as MatShape>::Cols, Head::Dyn, Tail::Dyn>;

    #[inline(always)]
    unsafe fn from_dyn_idx(idx: <Self::Dyn as MaybeContiguous>::Index) -> Self::Index {
        Head::from_dyn_idx(idx)
    }

    #[inline(always)]
    unsafe fn get_unchecked(this: &mut Self, index: Self::Index) -> Self::Item {
        Zip(
            Head::get_unchecked(&mut this.0, index),
            Tail::get_unchecked(&mut this.1, index),
        )
    }

    #[inline(always)]
    unsafe fn next_unchecked(slice: &mut Self::Slice) -> Self::Item {
        Zip(
            Head::next_unchecked(&mut slice.0),
            Tail::next_unchecked(&mut slice.1),
        )
    }

    #[inline(always)]
    fn is_contiguous(this: &Self) -> bool {
        Head::is_contiguous(&this.0) && Tail::is_contiguous(&this.1)
    }
    #[inline(always)]
    fn preferred_layout(this: &Self) -> Self::LayoutTransform {
        Head::preferred_layout(&this.0)
    }
    #[inline(always)]
    fn with_layout(this: Self, layout: Self::LayoutTransform) -> Self::Dyn {
        ZipEq(
            Head::with_layout(this.0, layout),
            Tail::with_layout(this.1, layout),
        )
    }
}

unsafe impl<E: Entity, R: Shape> MaybeContiguous for ColRef<'_, E, R> {
    type Index = Idx<R>;
    type Slice = GroupFor<E, &'static [MaybeUninit<E::Unit>]>;
    type LayoutTransform = VecLayoutTransform;

    #[inline(always)]
    unsafe fn get_slice_unchecked(this: &mut Self, i: Self::Index, n_elems: usize) -> Self::Slice {
        E::faer_map(
            (*this).rb().ptr_inbounds_at(i),
            #[inline(always)]
            |ptr| core::slice::from_raw_parts(ptr as *const MaybeUninit<E::Unit>, n_elems),
        )
    }
}
unsafe impl<'a, E: Entity, R: Shape> MatIndex for ColRef<'a, E, R> {
    type Item = Read<'a, E>;
    type RefItem = Ref<'a, E>;
    type Dyn = ColRef<'a, E>;

    #[inline(always)]
    unsafe fn to_ref(item: Self::Item) -> Self::RefItem {
        E::faer_map(
            item.ptr,
            #[inline(always)]
            |x| unsafe { &*(x as *const MaybeUninit<E::Unit> as *const E::Unit) },
        )
    }

    #[inline(always)]
    unsafe fn from_dyn_idx(idx: <Self::Dyn as MaybeContiguous>::Index) -> Self::Index {
        Idx::<R>::new_unbound(idx)
    }

    #[inline(always)]
    unsafe fn get_unchecked(this: &mut Self, i: Self::Index) -> Self::Item {
        Read {
            ptr: E::faer_map(
                this.rb().ptr_inbounds_at(i),
                #[inline(always)]
                |ptr| &*(ptr as *const MaybeUninit<E::Unit>),
            ),
        }
    }

    #[inline(always)]
    unsafe fn next_unchecked(slice: &mut Self::Slice) -> Self::Item {
        let (head, tail) = E::faer_unzip(E::faer_map(
            E::faer_as_mut(slice),
            #[inline(always)]
            |slice| core::mem::take(slice).split_first().unwrap_unchecked(),
        ));
        *slice = tail;
        Read { ptr: head }
    }

    #[inline(always)]
    fn is_contiguous(this: &Self) -> bool {
        this.row_stride() == 1
    }
    #[inline(always)]
    fn preferred_layout(this: &Self) -> Self::LayoutTransform {
        let rs = this.row_stride();
        let nrows = this.nrows().unbound();
        if nrows > 1 && rs == 1 {
            VecLayoutTransform::None
        } else if nrows > 1 && rs == -1 {
            VecLayoutTransform::Reverse
        } else {
            VecLayoutTransform::None
        }
    }
    #[inline(always)]
    fn with_layout(this: Self, layout: Self::LayoutTransform) -> Self::Dyn {
        use VecLayoutTransform::*;
        match layout {
            None => this.as_dyn(),
            Reverse => this.as_dyn().reverse_rows(),
        }
    }
}

unsafe impl<E: Entity, R: Shape> MaybeContiguous for ColMut<'_, E, R> {
    type Index = Idx<R>;
    type Slice = GroupFor<E, &'static mut [MaybeUninit<E::Unit>]>;
    type LayoutTransform = VecLayoutTransform;

    #[inline(always)]
    unsafe fn get_slice_unchecked(this: &mut Self, i: Self::Index, n_elems: usize) -> Self::Slice {
        E::faer_map(
            (*this).rb_mut().ptr_inbounds_at_mut(i),
            #[inline(always)]
            |ptr| core::slice::from_raw_parts_mut(ptr as *mut MaybeUninit<E::Unit>, n_elems),
        )
    }
}
unsafe impl<'a, E: Entity, R: Shape> MatIndex for ColMut<'a, E, R> {
    type Item = ReadWrite<'a, E>;
    type RefItem = Mut<'a, E>;
    type Dyn = ColMut<'a, E>;

    #[inline(always)]
    unsafe fn to_ref(item: Self::Item) -> Self::RefItem {
        E::faer_map(
            item.ptr,
            #[inline(always)]
            |x| unsafe { &mut *(x as *mut MaybeUninit<E::Unit> as *mut E::Unit) },
        )
    }

    #[inline(always)]
    unsafe fn from_dyn_idx(idx: <Self::Dyn as MaybeContiguous>::Index) -> Self::Index {
        Idx::<R>::new_unbound(idx)
    }

    #[inline(always)]
    unsafe fn get_unchecked(this: &mut Self, i: Self::Index) -> Self::Item {
        ReadWrite {
            ptr: E::faer_map(
                this.rb_mut().ptr_inbounds_at_mut(i),
                #[inline(always)]
                |ptr| &mut *(ptr as *mut MaybeUninit<E::Unit>),
            ),
        }
    }

    #[inline(always)]
    unsafe fn next_unchecked(slice: &mut Self::Slice) -> Self::Item {
        let (head, tail) = E::faer_unzip(E::faer_map(
            E::faer_as_mut(slice),
            #[inline(always)]
            |slice| core::mem::take(slice).split_first_mut().unwrap_unchecked(),
        ));
        *slice = tail;
        ReadWrite { ptr: head }
    }

    #[inline(always)]
    fn is_contiguous(this: &Self) -> bool {
        this.row_stride() == 1
    }
    #[inline(always)]
    fn preferred_layout(this: &Self) -> Self::LayoutTransform {
        let rs = this.row_stride();
        let nrows = this.nrows().unbound();
        if nrows > 1 && rs == 1 {
            VecLayoutTransform::None
        } else if nrows > 1 && rs == -1 {
            VecLayoutTransform::Reverse
        } else {
            VecLayoutTransform::None
        }
    }
    #[inline(always)]
    fn with_layout(this: Self, layout: Self::LayoutTransform) -> Self::Dyn {
        use VecLayoutTransform::*;
        match layout {
            None => this.as_dyn_mut(),
            Reverse => this.as_dyn_mut().reverse_rows_mut(),
        }
    }
}

unsafe impl<E: Entity, C: Shape> MaybeContiguous for RowRef<'_, E, C> {
    type Index = Idx<C>;
    type Slice = GroupFor<E, &'static [MaybeUninit<E::Unit>]>;
    type LayoutTransform = VecLayoutTransform;

    #[inline(always)]
    unsafe fn get_slice_unchecked(this: &mut Self, j: Self::Index, n_elems: usize) -> Self::Slice {
        E::faer_map(
            (*this).rb().ptr_inbounds_at(j),
            #[inline(always)]
            |ptr| core::slice::from_raw_parts(ptr as *const MaybeUninit<E::Unit>, n_elems),
        )
    }
}
unsafe impl<'a, E: Entity, C: Shape> MatIndex for RowRef<'a, E, C> {
    type Item = Read<'a, E>;
    type RefItem = Ref<'a, E>;
    type Dyn = RowRef<'a, E>;

    #[inline(always)]
    unsafe fn to_ref(item: Self::Item) -> Self::RefItem {
        E::faer_map(
            item.ptr,
            #[inline(always)]
            |x| unsafe { &*(x as *const MaybeUninit<E::Unit> as *const E::Unit) },
        )
    }

    #[inline(always)]
    unsafe fn from_dyn_idx(idx: <Self::Dyn as MaybeContiguous>::Index) -> Self::Index {
        Idx::<C>::new_unbound(idx)
    }

    #[inline(always)]
    unsafe fn get_unchecked(this: &mut Self, j: Self::Index) -> Self::Item {
        Read {
            ptr: E::faer_map(
                this.rb().ptr_inbounds_at(j),
                #[inline(always)]
                |ptr| &*(ptr as *const MaybeUninit<E::Unit>),
            ),
        }
    }

    #[inline(always)]
    unsafe fn next_unchecked(slice: &mut Self::Slice) -> Self::Item {
        let (head, tail) = E::faer_unzip(E::faer_map(
            E::faer_as_mut(slice),
            #[inline(always)]
            |slice| core::mem::take(slice).split_first().unwrap_unchecked(),
        ));
        *slice = tail;
        Read { ptr: head }
    }

    #[inline(always)]
    fn is_contiguous(this: &Self) -> bool {
        this.col_stride() == 1
    }
    #[inline(always)]
    fn preferred_layout(this: &Self) -> Self::LayoutTransform {
        let cs = this.col_stride();
        let ncols = this.ncols().unbound();

        if ncols > 1 && cs == 1 {
            VecLayoutTransform::None
        } else if ncols > 1 && cs == -1 {
            VecLayoutTransform::Reverse
        } else {
            VecLayoutTransform::None
        }
    }
    #[inline(always)]
    fn with_layout(this: Self, layout: Self::LayoutTransform) -> Self::Dyn {
        use VecLayoutTransform::*;
        match layout {
            None => this.as_dyn(),
            Reverse => this.as_dyn().reverse_cols(),
        }
    }
}

unsafe impl<E: Entity, C: Shape> MaybeContiguous for RowMut<'_, E, C> {
    type Index = Idx<C>;
    type Slice = GroupFor<E, &'static mut [MaybeUninit<E::Unit>]>;
    type LayoutTransform = VecLayoutTransform;

    #[inline(always)]
    unsafe fn get_slice_unchecked(this: &mut Self, j: Self::Index, n_elems: usize) -> Self::Slice {
        E::faer_map(
            (*this).rb_mut().ptr_inbounds_at_mut(j),
            #[inline(always)]
            |ptr| core::slice::from_raw_parts_mut(ptr as *mut MaybeUninit<E::Unit>, n_elems),
        )
    }
}
unsafe impl<'a, E: Entity, C: Shape> MatIndex for RowMut<'a, E, C> {
    type Item = ReadWrite<'a, E>;
    type RefItem = Mut<'a, E>;
    type Dyn = RowMut<'a, E>;

    #[inline(always)]
    unsafe fn to_ref(item: Self::Item) -> Self::RefItem {
        E::faer_map(
            item.ptr,
            #[inline(always)]
            |x| unsafe { &mut *(x as *mut MaybeUninit<E::Unit> as *mut E::Unit) },
        )
    }

    #[inline(always)]
    unsafe fn from_dyn_idx(idx: <Self::Dyn as MaybeContiguous>::Index) -> Self::Index {
        Idx::<C>::new_unbound(idx)
    }

    #[inline(always)]
    unsafe fn next_unchecked(slice: &mut Self::Slice) -> Self::Item {
        let (head, tail) = E::faer_unzip(E::faer_map(
            E::faer_as_mut(slice),
            #[inline(always)]
            |slice| core::mem::take(slice).split_first_mut().unwrap_unchecked(),
        ));
        *slice = tail;
        ReadWrite { ptr: head }
    }

    #[inline(always)]
    unsafe fn get_unchecked(this: &mut Self, j: Self::Index) -> Self::Item {
        ReadWrite {
            ptr: E::faer_map(
                this.rb_mut().ptr_inbounds_at_mut(j),
                #[inline(always)]
                |ptr| &mut *(ptr as *mut MaybeUninit<E::Unit>),
            ),
        }
    }

    #[inline(always)]
    fn is_contiguous(this: &Self) -> bool {
        this.col_stride() == 1
    }
    #[inline(always)]
    fn preferred_layout(this: &Self) -> Self::LayoutTransform {
        let cs = this.col_stride();
        let ncols = this.ncols().unbound();
        if ncols > 1 && cs == 1 {
            VecLayoutTransform::None
        } else if ncols > 1 && cs == -1 {
            VecLayoutTransform::Reverse
        } else {
            VecLayoutTransform::None
        }
    }
    #[inline(always)]
    fn with_layout(this: Self, layout: Self::LayoutTransform) -> Self::Dyn {
        use VecLayoutTransform::*;
        match layout {
            None => this.as_dyn_mut(),
            Reverse => this.as_dyn_mut().reverse_cols_mut(),
        }
    }
}

unsafe impl<E: Entity, R: Shape, C: Shape> MaybeContiguous for MatRef<'_, E, R, C> {
    type Index = (Idx<R>, Idx<C>);
    type Slice = GroupFor<E, &'static [MaybeUninit<E::Unit>]>;
    type LayoutTransform = MatLayoutTransform;

    #[inline(always)]
    unsafe fn get_slice_unchecked(
        this: &mut Self,
        (i, j): Self::Index,
        n_elems: usize,
    ) -> Self::Slice {
        E::faer_map(
            (*this).rb().ptr_inbounds_at(i.into(), j.into()),
            #[inline(always)]
            |ptr| core::slice::from_raw_parts(ptr as *const MaybeUninit<E::Unit>, n_elems),
        )
    }
}
unsafe impl<'a, E: Entity, R: Shape, C: Shape> MatIndex for MatRef<'a, E, R, C> {
    type Item = Read<'a, E>;
    type RefItem = Ref<'a, E>;
    type Dyn = MatRef<'a, E>;

    #[inline(always)]
    unsafe fn to_ref(item: Self::Item) -> Self::RefItem {
        E::faer_map(
            item.ptr,
            #[inline(always)]
            |x| unsafe { &*(x as *const MaybeUninit<E::Unit> as *const E::Unit) },
        )
    }

    #[inline(always)]
    unsafe fn from_dyn_idx(idx: <Self::Dyn as MaybeContiguous>::Index) -> Self::Index {
        (Idx::<R>::new_unbound(idx.0), Idx::<C>::new_unbound(idx.1))
    }

    #[inline(always)]
    unsafe fn get_unchecked(this: &mut Self, (i, j): Self::Index) -> Self::Item {
        Read {
            ptr: E::faer_map(
                this.rb().ptr_inbounds_at(i, j),
                #[inline(always)]
                |ptr| &*(ptr as *const MaybeUninit<E::Unit>),
            ),
        }
    }

    #[inline(always)]
    unsafe fn next_unchecked(slice: &mut Self::Slice) -> Self::Item {
        let (head, tail) = E::faer_unzip(E::faer_map(
            E::faer_as_mut(slice),
            #[inline(always)]
            |slice| core::mem::take(slice).split_first().unwrap_unchecked(),
        ));
        *slice = tail;
        Read { ptr: head }
    }

    #[inline(always)]
    fn is_contiguous(this: &Self) -> bool {
        this.row_stride() == 1
    }
    #[inline(always)]
    fn preferred_layout(this: &Self) -> Self::LayoutTransform {
        let rs = this.row_stride();
        let cs = this.col_stride();
        let nrows = this.nrows().unbound();
        let ncols = this.ncols().unbound();

        if nrows > 1 && rs == 1 {
            MatLayoutTransform::None
        } else if nrows > 1 && rs == -1 {
            MatLayoutTransform::ReverseRows
        } else if ncols > 1 && cs == 1 {
            MatLayoutTransform::Transpose
        } else if ncols > 1 && cs == -1 {
            MatLayoutTransform::TransposeReverseRows
        } else {
            MatLayoutTransform::None
        }
    }
    #[inline(always)]
    fn with_layout(this: Self, layout: Self::LayoutTransform) -> Self::Dyn {
        use MatLayoutTransform::*;
        match layout {
            None => this.as_dyn(),
            ReverseRows => this.as_dyn().reverse_rows(),
            Transpose => this.as_dyn().transpose(),
            TransposeReverseRows => this.as_dyn().transpose().reverse_rows(),
        }
    }
}

unsafe impl<E: Entity, R: Shape, C: Shape> MaybeContiguous for MatMut<'_, E, R, C> {
    type Index = (Idx<R>, Idx<C>);
    type Slice = GroupFor<E, &'static mut [MaybeUninit<E::Unit>]>;
    type LayoutTransform = MatLayoutTransform;

    #[inline(always)]
    unsafe fn get_slice_unchecked(
        this: &mut Self,
        (i, j): Self::Index,
        n_elems: usize,
    ) -> Self::Slice {
        E::faer_map(
            (*this).rb().ptr_inbounds_at(i, j),
            #[inline(always)]
            |ptr| core::slice::from_raw_parts_mut(ptr as *mut MaybeUninit<E::Unit>, n_elems),
        )
    }
}

unsafe impl<'a, E: Entity, R: Shape, C: Shape> MatIndex for MatMut<'a, E, R, C> {
    type Item = ReadWrite<'a, E>;
    type RefItem = Mut<'a, E>;
    type Dyn = MatMut<'a, E>;

    #[inline(always)]
    unsafe fn to_ref(item: Self::Item) -> Self::RefItem {
        E::faer_map(
            item.ptr,
            #[inline(always)]
            |x| unsafe { &mut *(x as *mut MaybeUninit<E::Unit> as *mut E::Unit) },
        )
    }

    #[inline(always)]
    unsafe fn from_dyn_idx(idx: <Self::Dyn as MaybeContiguous>::Index) -> Self::Index {
        (Idx::<R>::new_unbound(idx.0), Idx::<C>::new_unbound(idx.1))
    }

    #[inline(always)]
    unsafe fn get_unchecked(this: &mut Self, (i, j): Self::Index) -> Self::Item {
        ReadWrite {
            ptr: E::faer_map(
                this.rb_mut().ptr_inbounds_at_mut(i, j),
                #[inline(always)]
                |ptr| &mut *(ptr as *mut MaybeUninit<E::Unit>),
            ),
        }
    }

    #[inline(always)]
    unsafe fn next_unchecked(slice: &mut Self::Slice) -> Self::Item {
        let (head, tail) = E::faer_unzip(E::faer_map(
            E::faer_as_mut(slice),
            #[inline(always)]
            |slice| core::mem::take(slice).split_first_mut().unwrap_unchecked(),
        ));
        *slice = tail;
        ReadWrite { ptr: head }
    }

    #[inline(always)]
    fn is_contiguous(this: &Self) -> bool {
        this.row_stride() == 1
    }
    #[inline(always)]
    fn preferred_layout(this: &Self) -> Self::LayoutTransform {
        let rs = this.row_stride();
        let cs = this.col_stride();
        let nrows = this.nrows().unbound();
        let ncols = this.ncols().unbound();

        if nrows > 1 && rs == 1 {
            MatLayoutTransform::None
        } else if nrows > 1 && rs == -1 {
            MatLayoutTransform::ReverseRows
        } else if ncols > 1 && cs == 1 {
            MatLayoutTransform::Transpose
        } else if ncols > 1 && cs == -1 {
            MatLayoutTransform::TransposeReverseRows
        } else {
            MatLayoutTransform::None
        }
    }
    #[inline(always)]
    fn with_layout(this: Self, layout: Self::LayoutTransform) -> Self::Dyn {
        use MatLayoutTransform::*;
        match layout {
            None => this.as_dyn_mut(),
            ReverseRows => this.as_dyn_mut().reverse_rows_mut(),
            Transpose => this.as_dyn_mut().transpose_mut(),
            TransposeReverseRows => this.as_dyn_mut().transpose_mut().reverse_rows_mut(),
        }
    }
}

#[inline(always)]
fn annotate_noalias_mat<Z: MatIndex>(
    f: &mut impl FnMut(<Z as MatIndex>::Item),
    mut slice: Z::Slice,
    i_begin: usize,
    i_end: usize,
    _j: usize,
) {
    for _ in i_begin..i_end {
        unsafe { f(Z::next_unchecked(&mut slice)) };
    }
}

#[inline(always)]
fn annotate_noalias_mat_with_index<
    Z: MatIndex<Index = (RowIdx, ColIdx), Dyn: MatIndex<Index = (usize, usize)>>,
    RowIdx,
    ColIdx,
>(
    f: &mut impl FnMut(RowIdx, ColIdx, <Z as MatIndex>::Item),
    mut slice: Z::Slice,
    i_begin: usize,
    i_end: usize,
    j: usize,
    transpose: bool,
    reverse_rows: bool,
) {
    if !transpose {
        if !reverse_rows {
            for i in i_begin..i_end {
                unsafe {
                    let (ii, jj) = Z::from_dyn_idx((i, j));
                    f(ii, jj, Z::next_unchecked(&mut slice))
                };
            }
        } else {
            for i in i_begin..i_end {
                unsafe {
                    let (ii, jj) = Z::from_dyn_idx((i_begin + (i_end - i - 1), j));
                    f(ii, jj, Z::next_unchecked(&mut slice))
                };
            }
        }
    } else {
        if !reverse_rows {
            for i in i_begin..i_end {
                unsafe {
                    let (ii, jj) = Z::from_dyn_idx((j, i));
                    f(ii, jj, Z::next_unchecked(&mut slice))
                };
            }
        } else {
            for i in i_begin..i_end {
                unsafe {
                    let (ii, jj) = Z::from_dyn_idx((j, i_begin + (i_end - i - 1)));
                    f(ii, jj, Z::next_unchecked(&mut slice))
                };
            }
        }
    }
}

#[inline(always)]
fn annotate_noalias_col<Z: MatIndex>(
    f: &mut impl FnMut(<Z as MatIndex>::Item),
    mut slice: Z::Slice,
    i_begin: usize,
    i_end: usize,
) {
    for _ in i_begin..i_end {
        unsafe { f(Z::next_unchecked(&mut slice)) };
    }
}

#[inline(always)]
fn annotate_noalias_col_with_index<
    Z: MatIndex<Index = Idx, Dyn: MatIndex<Item = Z::Item, Index = usize>>,
    Idx,
>(
    f: &mut impl FnMut(Idx, <Z as MatIndex>::Item),
    mut slice: Z::Slice,
    i_begin: usize,
    i_end: usize,
    reverse: bool,
) {
    if !reverse {
        for i in i_begin..i_end {
            unsafe {
                let ii = Z::from_dyn_idx(i);
                f(ii, Z::next_unchecked(&mut slice))
            };
        }
    } else {
        for i in i_begin..i_end {
            unsafe {
                let ii = Z::from_dyn_idx(i_begin + (i_end - i - 1));
                f(ii, Z::next_unchecked(&mut slice))
            };
        }
    }
}

#[inline(always)]
fn for_each_mat<
    Z: MatIndex<
        Dyn: MatIndex<
            Item = Z::Item,
            Slice = Z::Slice,
            Rows = usize,
            Cols = usize,
            Index = (usize, usize),
        >,
    >,
>(
    z: Z,
    mut f: impl FnMut(<Z as MatIndex>::Item),
) {
    let layout = Z::preferred_layout(&z);
    let mut z = Z::with_layout(z, layout);

    let m = Z::Dyn::nrows(&z);
    let n = Z::Dyn::ncols(&z);
    if m == 0 || n == 0 {
        return;
    }

    unsafe {
        if Z::Dyn::is_contiguous(&z) {
            for j in 0..n {
                annotate_noalias_mat::<Z::Dyn>(
                    &mut f,
                    Z::Dyn::get_slice_unchecked(&mut z, (0, j), m),
                    0,
                    m,
                    j,
                );
            }
        } else {
            for j in 0..n {
                for i in 0..m {
                    f(Z::Dyn::get_unchecked(&mut z, (i, j)))
                }
            }
        }
    }
}

// TODO:
// - for_each_vec_with_index

#[inline(always)]
fn for_each_mat_with_index<
    RowIdx,
    ColIdx,
    Z: MatIndex<
        Index = (RowIdx, ColIdx),
        Dyn: MatIndex<
            Rows = usize,
            Cols = usize,
            Index = (usize, usize),
            Slice = Z::Slice,
            Item = Z::Item,
        >,
        LayoutTransform = MatLayoutTransform,
    >,
>(
    z: Z,
    mut f: impl FnMut(RowIdx, ColIdx, <Z as MatIndex>::Item),
) {
    let layout = Z::preferred_layout(&z);
    let mut z = Z::with_layout(z, layout);

    let m = Z::Dyn::nrows(&z);
    let n = Z::Dyn::ncols(&z);
    if m == 0 || n == 0 {
        return;
    }

    match layout {
        MatLayoutTransform::None => unsafe {
            if Z::Dyn::is_contiguous(&z) {
                for j in 0..n {
                    annotate_noalias_mat_with_index::<Z, _, _>(
                        &mut f,
                        Z::Dyn::get_slice_unchecked(&mut z, (0, j), m),
                        0,
                        m,
                        j,
                        false,
                        false,
                    );
                }
            } else {
                for j in 0..n {
                    for i in 0..m {
                        let (ii, jj) = Z::from_dyn_idx((i, j));
                        f(ii, jj, Z::Dyn::get_unchecked(&mut z, (i, j)))
                    }
                }
            }
        },
        MatLayoutTransform::ReverseRows => unsafe {
            if Z::Dyn::is_contiguous(&z) {
                for j in 0..n {
                    annotate_noalias_mat_with_index::<Z, _, _>(
                        &mut f,
                        Z::Dyn::get_slice_unchecked(&mut z, (0, j), m),
                        0,
                        m,
                        j,
                        false,
                        true,
                    );
                }
            } else {
                for j in 0..n {
                    for i in 0..m {
                        let (ii, jj) = Z::from_dyn_idx((m - i - 1, j));
                        f(ii, jj, Z::Dyn::get_unchecked(&mut z, (i, j)))
                    }
                }
            }
        },
        MatLayoutTransform::Transpose => unsafe {
            if Z::Dyn::is_contiguous(&z) {
                for j in 0..n {
                    annotate_noalias_mat_with_index::<Z, _, _>(
                        &mut f,
                        Z::Dyn::get_slice_unchecked(&mut z, (0, j), m),
                        0,
                        m,
                        j,
                        true,
                        false,
                    );
                }
            } else {
                for j in 0..n {
                    for i in 0..m {
                        let (ii, jj) = Z::from_dyn_idx((j, i));
                        f(ii, jj, Z::Dyn::get_unchecked(&mut z, (i, j)))
                    }
                }
            }
        },
        MatLayoutTransform::TransposeReverseRows => unsafe {
            if Z::Dyn::is_contiguous(&z) {
                for j in 0..n {
                    annotate_noalias_mat_with_index::<Z, _, _>(
                        &mut f,
                        Z::Dyn::get_slice_unchecked(&mut z, (0, j), m),
                        0,
                        m,
                        j,
                        true,
                        true,
                    );
                }
            } else {
                for j in 0..n {
                    for i in 0..m {
                        let (ii, jj) = Z::from_dyn_idx((j, m - i - 1));
                        f(ii, jj, Z::Dyn::get_unchecked(&mut z, (i, j)))
                    }
                }
            }
        },
    }
}

#[inline(always)]
fn for_each_mat_triangular_lower_with_index<
    RowIdx,
    ColIdx,
    Z: MatIndex<
        Index = (RowIdx, ColIdx),
        Dyn: MatIndex<
            Rows = usize,
            Cols = usize,
            Index = (usize, usize),
            Item = Z::Item,
            Slice = Z::Slice,
        >,
        LayoutTransform = MatLayoutTransform,
    >,
>(
    z: Z,
    diag: Diag,
    mut f: impl FnMut(RowIdx, ColIdx, <Z as MatIndex>::Item),
) {
    let layout = Z::preferred_layout(&z);
    let mut z = Z::with_layout(z, layout);

    let m = Z::Dyn::nrows(&z);
    let n = Z::Dyn::ncols(&z);
    let strict = match diag {
        Diag::Skip => true,
        Diag::Include => false,
    };
    let strict = strict as usize;

    if m == 0 || n == 0 {
        return;
    }

    match layout {
        MatLayoutTransform::None => unsafe {
            if Z::Dyn::is_contiguous(&z) {
                for j in 0..n {
                    let start = j + strict;
                    let end = m;
                    if start == end {
                        continue;
                    }
                    annotate_noalias_mat_with_index::<Z, _, _>(
                        &mut f,
                        Z::Dyn::get_slice_unchecked(&mut z, (start, j), end - start),
                        start,
                        end,
                        j,
                        false,
                        false,
                    );
                }
            } else {
                for j in 0..n {
                    let start = j + strict;
                    let end = m;
                    if start == end {
                        continue;
                    }
                    for i in start..end {
                        let (ii, jj) = Z::from_dyn_idx((i, j));
                        f(ii, jj, Z::Dyn::get_unchecked(&mut z, (i, j)))
                    }
                }
            }
        },
        MatLayoutTransform::ReverseRows => unsafe {
            if Z::Dyn::is_contiguous(&z) {
                for j in 0..Ord::min(m, n) {
                    let start = 0;
                    let end = m - j - strict;
                    if start == end {
                        continue;
                    }
                    annotate_noalias_mat_with_index::<Z, _, _>(
                        &mut f,
                        Z::Dyn::get_slice_unchecked(&mut z, (start, j), end - start),
                        j + strict + start,
                        j + strict + end,
                        j,
                        false,
                        true,
                    );
                }
            } else {
                for j in 0..Ord::min(m, n) {
                    let start = 0;
                    let end = m - j - strict;
                    if start == end {
                        continue;
                    }
                    for i in start..end {
                        let (ii, jj) = Z::from_dyn_idx((m - i - 1, j));
                        f(ii, jj, Z::Dyn::get_unchecked(&mut z, (i, j)))
                    }
                }
            }
        },
        MatLayoutTransform::Transpose => unsafe {
            if Z::Dyn::is_contiguous(&z) {
                for j in 0..n {
                    let start = 0;
                    let end = Ord::min(m, j + (1 - strict));
                    if start == end {
                        continue;
                    }
                    annotate_noalias_mat_with_index::<Z, _, _>(
                        &mut f,
                        Z::Dyn::get_slice_unchecked(&mut z, (0, j), end - start),
                        start,
                        end,
                        j,
                        true,
                        false,
                    );
                }
            } else {
                for j in 0..n {
                    let start = 0;
                    let end = Ord::min(m, j + (1 - strict));
                    if start == end {
                        continue;
                    }
                    for i in start..end {
                        let (ii, jj) = Z::from_dyn_idx((j, i));
                        f(ii, jj, Z::Dyn::get_unchecked(&mut z, (i, j)))
                    }
                }
            }
        },
        MatLayoutTransform::TransposeReverseRows => unsafe {
            if Z::Dyn::is_contiguous(&z) {
                for j in 0..n {
                    let start = m - Ord::min(j + (1 - strict) as usize, m);
                    let end = m;
                    if start == end {
                        continue;
                    }
                    annotate_noalias_mat_with_index::<Z, _, _>(
                        &mut f,
                        Z::Dyn::get_slice_unchecked(&mut z, (start, j), end - start),
                        0,
                        end - start,
                        j,
                        true,
                        true,
                    );
                }
            } else {
                for j in 0..n {
                    let start = m - Ord::min(j + (1 - strict) as usize, m);
                    let end = m;
                    if start == end {
                        continue;
                    }
                    for i in start..end {
                        let (ii, jj) = Z::from_dyn_idx((j, m - i - 1));
                        f(ii, jj, Z::Dyn::get_unchecked(&mut z, (i, j)))
                    }
                }
            }
        },
    }
}

#[inline(always)]
fn for_each_mat_triangular_upper_with_index<
    RowIdx,
    ColIdx,
    Z: MatIndex<
        Index = (RowIdx, ColIdx),
        Dyn: MatIndex<
            Rows = usize,
            Cols = usize,
            Index = (usize, usize),
            Item = Z::Item,
            Slice = Z::Slice,
        >,
        LayoutTransform = MatLayoutTransform,
    >,
>(
    z: Z,
    diag: Diag,
    mut f: impl FnMut(RowIdx, ColIdx, <Z as MatIndex>::Item),
) {
    let layout = Z::preferred_layout(&z);
    let mut z = Z::with_layout(z, layout);

    let m = Z::Dyn::nrows(&z);
    let n = Z::Dyn::ncols(&z);
    let strict = match diag {
        Diag::Skip => true,
        Diag::Include => false,
    };
    let strict = strict as usize;

    if m == 0 || n == 0 {
        return;
    }

    match layout {
        MatLayoutTransform::None => unsafe {
            if Z::Dyn::is_contiguous(&z) {
                for j in 0..n {
                    let start = 0;
                    let end = Ord::min(m, j + (1 - strict));
                    if start == end {
                        continue;
                    }

                    annotate_noalias_mat_with_index::<Z, _, _>(
                        &mut f,
                        Z::Dyn::get_slice_unchecked(&mut z, (start, j), end - start),
                        start,
                        end,
                        j,
                        false,
                        false,
                    );
                }
            } else {
                for j in 0..n {
                    let start = 0;
                    let end = Ord::min(m, j + (1 - strict));
                    if start == end {
                        continue;
                    }
                    for i in start..end {
                        let (ii, jj) = Z::from_dyn_idx((i, j));
                        f(ii, jj, Z::Dyn::get_unchecked(&mut z, (i, j)))
                    }
                }
            }
        },
        MatLayoutTransform::ReverseRows => unsafe {
            if Z::Dyn::is_contiguous(&z) {
                for j in 0..Ord::min(m, n) {
                    let start = m - Ord::min(j + (1 - strict) as usize, m);
                    let end = m;
                    if start == end {
                        continue;
                    }
                    annotate_noalias_mat_with_index::<Z, _, _>(
                        &mut f,
                        Z::Dyn::get_slice_unchecked(&mut z, (start, j), end - start),
                        0,
                        end - start,
                        j,
                        false,
                        true,
                    );
                }
            } else {
                for j in 0..Ord::min(m, n) {
                    let start = m - Ord::min(j + (1 - strict) as usize, m);
                    let end = m;
                    if start == end {
                        continue;
                    }
                    for i in start..end {
                        let (ii, jj) = Z::from_dyn_idx((m - i - 1, j));
                        f(ii, jj, Z::Dyn::get_unchecked(&mut z, (i, j)))
                    }
                }
            }
        },
        MatLayoutTransform::Transpose => unsafe {
            if Z::Dyn::is_contiguous(&z) {
                for j in 0..n {
                    let start = j + strict;
                    let end = m;
                    if start == end {
                        continue;
                    }
                    annotate_noalias_mat_with_index::<Z, _, _>(
                        &mut f,
                        Z::Dyn::get_slice_unchecked(&mut z, (start, j), end - start),
                        start,
                        end,
                        j,
                        true,
                        false,
                    );
                }
            } else {
                for j in 0..n {
                    let start = j + strict;
                    let end = m;
                    if start == end {
                        continue;
                    }
                    for i in start..end {
                        let (ii, jj) = Z::from_dyn_idx((j, i));
                        f(ii, jj, Z::Dyn::get_unchecked(&mut z, (i, j)))
                    }
                }
            }
        },
        MatLayoutTransform::TransposeReverseRows => unsafe {
            if Z::Dyn::is_contiguous(&z) {
                for j in 0..n {
                    let start = 0;
                    let end = m - j - strict;
                    if start == end {
                        continue;
                    }
                    annotate_noalias_mat_with_index::<Z, _, _>(
                        &mut f,
                        Z::Dyn::get_slice_unchecked(&mut z, (start, j), end - start),
                        j + strict,
                        j + strict + end - start,
                        j,
                        true,
                        true,
                    );
                }
            } else {
                for j in 0..n {
                    let start = 0;
                    let end = m - j - strict;
                    if start == end {
                        continue;
                    }
                    for i in start..end {
                        let (ii, jj) = Z::from_dyn_idx((j, m - i - 1));
                        f(ii, jj, Z::Dyn::get_unchecked(&mut z, (i, j)))
                    }
                }
            }
        },
    }
}

#[inline(always)]
fn for_each_mat_triangular_lower<
    Z: MatIndex<
        LayoutTransform = MatLayoutTransform,
        Dyn: MatIndex<
            LayoutTransform = MatLayoutTransform,
            Item = Z::Item,
            Slice = Z::Slice,
            Rows = usize,
            Cols = usize,
            Index = (usize, usize),
            Dyn = Z::Dyn,
        >,
    >,
>(
    z: Z,
    diag: Diag,
    transpose: bool,
    mut f: impl FnMut(<Z as MatIndex>::Item),
) {
    use MatLayoutTransform::*;

    let z = if transpose {
        Z::with_layout(z, MatLayoutTransform::Transpose)
    } else {
        Z::with_layout(z, MatLayoutTransform::None)
    };
    let layout = Z::Dyn::preferred_layout(&z);
    let mut z = Z::Dyn::with_layout(z, layout);

    let m = Z::Dyn::nrows(&z);
    let n = Z::Dyn::ncols(&z);
    let n = match layout {
        None | ReverseRows => Ord::min(m, n),
        Transpose | TransposeReverseRows => n,
    };
    if m == 0 || n == 0 {
        return;
    }

    let strict = match diag {
        Diag::Skip => true,
        Diag::Include => false,
    };

    unsafe {
        if Z::Dyn::is_contiguous(&z) {
            for j in 0..n {
                let (start, end) = match layout {
                    None => (j + strict as usize, m),
                    ReverseRows => (0, (m - (j + strict as usize))),
                    Transpose => (0, (j + !strict as usize).min(m)),
                    TransposeReverseRows => (m - ((j + !strict as usize).min(m)), m),
                };

                let len = end - start;
                if start == end {
                    continue;
                }

                annotate_noalias_mat::<Z::Dyn>(
                    &mut f,
                    Z::Dyn::get_slice_unchecked(&mut z, (start, j), len),
                    start,
                    end,
                    j,
                );
            }
        } else {
            for j in 0..n {
                let (start, end) = match layout {
                    None => (j + strict as usize, m),
                    ReverseRows => (0, (m - (j + strict as usize))),
                    Transpose => (0, (j + !strict as usize).min(m)),
                    TransposeReverseRows => (m - ((j + !strict as usize).min(m)), m),
                };
                if start == end {
                    continue;
                }

                for i in start..end {
                    f(Z::Dyn::get_unchecked(&mut z, (i, j)))
                }
            }
        }
    }
}

#[inline(always)]
fn for_each_col<
    Z: MatIndex<
        Dyn: MatIndex<Rows = usize, Cols = (), Index = usize, Item = Z::Item, Slice = Z::Slice>,
    >,
>(
    z: Z,
    mut f: impl FnMut(<Z as MatIndex>::Item),
) {
    let layout = Z::preferred_layout(&z);
    let mut z = Z::with_layout(z, layout);

    let m = Z::Dyn::nrows(&z);
    if m == 0 {
        return;
    }

    unsafe {
        if Z::Dyn::is_contiguous(&z) {
            annotate_noalias_col::<Z::Dyn>(&mut f, Z::Dyn::get_slice_unchecked(&mut z, 0, m), 0, m);
        } else {
            for i in 0..m {
                f(Z::Dyn::get_unchecked(&mut z, i))
            }
        }
    }
}

#[inline(always)]
fn for_each_col_with_index<
    Idx,
    Z: MatIndex<
        LayoutTransform = VecLayoutTransform,
        Index = Idx,
        Dyn: MatIndex<Rows = usize, Cols = (), Index = usize, Item = Z::Item, Slice = Z::Slice>,
    >,
>(
    z: Z,
    mut f: impl FnMut(Idx, <Z as MatIndex>::Item),
) {
    let layout = Z::preferred_layout(&z);
    let mut z = Z::with_layout(z, layout);

    let m = Z::Dyn::nrows(&z);
    if m == 0 {
        return;
    }

    unsafe {
        match layout {
            VecLayoutTransform::None => {
                if Z::Dyn::is_contiguous(&z) {
                    annotate_noalias_col_with_index::<Z, _>(
                        &mut f,
                        Z::Dyn::get_slice_unchecked(&mut z, 0, m),
                        0,
                        m,
                        false,
                    );
                } else {
                    for i in 0..m {
                        f(Z::from_dyn_idx(i), Z::Dyn::get_unchecked(&mut z, i))
                    }
                }
            }
            VecLayoutTransform::Reverse => {
                if Z::Dyn::is_contiguous(&z) {
                    annotate_noalias_col_with_index::<Z, _>(
                        &mut f,
                        Z::Dyn::get_slice_unchecked(&mut z, 0, m),
                        0,
                        m,
                        true,
                    );
                } else {
                    for i in 0..m {
                        f(Z::from_dyn_idx(m - i - 1), Z::Dyn::get_unchecked(&mut z, i))
                    }
                }
            }
        }
    }
}

#[inline(always)]
fn for_each_row_with_index<
    Idx,
    Z: MatIndex<
        LayoutTransform = VecLayoutTransform,
        Index = Idx,
        Dyn: MatIndex<Rows = (), Cols = usize, Index = usize, Item = Z::Item, Slice = Z::Slice>,
    >,
>(
    z: Z,
    mut f: impl FnMut(Idx, <Z as MatIndex>::Item),
) {
    let layout = Z::preferred_layout(&z);
    let mut z = Z::with_layout(z, layout);

    let n = Z::Dyn::ncols(&z);
    if n == 0 {
        return;
    }

    unsafe {
        match layout {
            VecLayoutTransform::None => {
                if Z::Dyn::is_contiguous(&z) {
                    annotate_noalias_col_with_index::<Z, _>(
                        &mut f,
                        Z::Dyn::get_slice_unchecked(&mut z, 0, n),
                        0,
                        n,
                        false,
                    );
                } else {
                    for i in 0..n {
                        f(Z::from_dyn_idx(i), Z::Dyn::get_unchecked(&mut z, i))
                    }
                }
            }
            VecLayoutTransform::Reverse => {
                if Z::Dyn::is_contiguous(&z) {
                    annotate_noalias_col_with_index::<Z, _>(
                        &mut f,
                        Z::Dyn::get_slice_unchecked(&mut z, 0, n),
                        0,
                        n,
                        true,
                    );
                } else {
                    for i in 0..n {
                        f(Z::from_dyn_idx(n - i - 1), Z::Dyn::get_unchecked(&mut z, i))
                    }
                }
            }
        }
    }
}
#[inline(always)]
fn for_each_row<
    Z: MatIndex<
        Dyn: MatIndex<Rows = (), Cols = usize, Index = usize, Item = Z::Item, Slice = Z::Slice>,
    >,
>(
    z: Z,
    mut f: impl FnMut(<Z as MatIndex>::Item),
) {
    let layout = Z::preferred_layout(&z);
    let mut z = Z::with_layout(z, layout);

    let n = Z::Dyn::ncols(&z);
    if n == 0 {
        return;
    }

    unsafe {
        if Z::Dyn::is_contiguous(&z) {
            annotate_noalias_col::<Z::Dyn>(&mut f, Z::Dyn::get_slice_unchecked(&mut z, 0, n), 0, n);
        } else {
            for j in 0..n {
                f(Z::Dyn::get_unchecked(&mut z, j))
            }
        }
    }
}

impl<
        R: Shape,
        C: Shape,
        M: MatIndex<
            LayoutTransform = MatLayoutTransform,
            Rows = R,
            Cols = C,
            Index = (Idx<R>, Idx<C>),
            Dyn: MatIndex<
                LayoutTransform = MatLayoutTransform,
                Item = M::Item,
                Slice = M::Slice,
                Rows = usize,
                Cols = usize,
                Index = (usize, usize),
                Dyn = M::Dyn,
            >,
        >,
    > LastEq<R, C, M>
{
    /// Applies `f` to each element of `self`.
    #[inline(always)]
    pub fn for_each(self, f: impl FnMut(<Self as MatIndex>::Item)) {
        for_each_mat(self, f);
    }

    /// Applies `f` to each element of `self`, while passing the indices of the position of the
    /// current element.
    #[inline(always)]
    pub fn for_each_with_index(self, f: impl FnMut(Idx<R>, Idx<C>, <Self as MatIndex>::Item)) {
        for_each_mat_with_index(self, f);
    }

    /// Applies `f` to each element of the lower triangular half of `self`, while passing the
    /// indices of the position of the current element.
    ///
    /// `diag` specifies whether the diagonal should be included or excluded.
    #[inline(always)]
    pub fn for_each_triangular_lower_with_index(
        self,
        diag: Diag,
        f: impl FnMut(Idx<R>, Idx<C>, <Self as MatIndex>::Item),
    ) {
        for_each_mat_triangular_lower_with_index(self, diag, f);
    }

    /// Applies `f` to each element of the upper triangular half of `self`, while passing the
    /// indices of the position of the current element.
    ///
    /// `diag` specifies whether the diagonal should be included or excluded.
    #[inline(always)]
    pub fn for_each_triangular_upper_with_index(
        self,
        diag: Diag,
        f: impl FnMut(Idx<R>, Idx<C>, <Self as MatIndex>::Item),
    ) {
        for_each_mat_triangular_upper_with_index(self, diag, f);
    }

    /// Applies `f` to each element of the lower triangular half of `self`.
    ///
    /// `diag` specifies whether the diagonal should be included or excluded.
    #[inline(always)]
    pub fn for_each_triangular_lower(self, diag: Diag, f: impl FnMut(<Self as MatIndex>::Item)) {
        for_each_mat_triangular_lower(self, diag, false, f);
    }

    /// Applies `f` to each element of the upper triangular half of `self`.
    ///
    /// `diag` specifies whether the diagonal should be included or excluded.
    #[inline(always)]
    pub fn for_each_triangular_upper(self, diag: Diag, f: impl FnMut(<Self as MatIndex>::Item)) {
        for_each_mat_triangular_lower(self, diag, true, f);
    }

    /// Applies `f` to each element of `self` and collect its result into a new matrix.
    #[inline(always)]
    pub fn map<E: Entity>(self, f: impl FnMut(<Self as MatIndex>::Item) -> E) -> Mat<E, R, C> {
        let (m, n) = (Self::nrows(&self), Self::ncols(&self));
        let mut out = Mat::<E>::with_capacity(m.unbound(), n.unbound());
        let rs = 1;
        let cs = out.col_stride();
        let out_view =
            unsafe { mat::from_raw_parts_mut::<'_, E, _, _>(out.as_ptr_mut(), m, n, rs, cs) };
        let mut f = f;
        ZipEq::new(out_view, self).for_each(
            #[inline(always)]
            |Zip(mut out, item)| out.write(f(item)),
        );
        unsafe { out.set_dims(m.unbound(), n.unbound()) };
        out.into_shape(m, n)
    }

    /// Applies `f` to each element of `self` and collect its result into a new matrix.
    #[inline(always)]
    pub fn map_with_index<E: Entity>(
        self,
        f: impl FnMut(Idx<R>, Idx<C>, <Self as MatIndex>::Item) -> E,
    ) -> Mat<E, R, C> {
        let (m, n) = (Self::nrows(&self), Self::ncols(&self));
        let mut out = Mat::<E>::with_capacity(m.unbound(), n.unbound());
        let rs = 1;
        let cs = out.col_stride();
        let out_view =
            unsafe { mat::from_raw_parts_mut::<'_, E, _, _>(out.as_ptr_mut(), m, n, rs, cs) };
        let mut f = f;
        ZipEq::new(out_view, self).for_each_with_index(
            #[inline(always)]
            |i, j, Zip(mut out, item)| out.write(f(i, j, item)),
        );
        unsafe { out.set_dims(m.unbound(), n.unbound()) };
        out.into_shape(m, n)
    }
}

impl<
        C: Shape,
        M: MatIndex<
            LayoutTransform = VecLayoutTransform,
            Rows = (),
            Cols = C,
            Index = Idx<C>,
            Dyn: MatIndex<
                LayoutTransform = VecLayoutTransform,
                Item = M::Item,
                Slice = M::Slice,
                Rows = (),
                Cols = usize,
                Index = usize,
                Dyn = M::Dyn,
            >,
        >,
    > LastEq<(), C, M>
{
    /// Applies `f` to each element of `self`.
    #[inline(always)]
    pub fn for_each(self, f: impl FnMut(<Self as MatIndex>::Item)) {
        for_each_row(self, f);
    }

    /// Applies `f` to each element of `self`, while passing in the index of the current element.
    #[inline(always)]
    pub fn for_each_with_index(self, f: impl FnMut(Idx<C>, <Self as MatIndex>::Item)) {
        for_each_row_with_index(self, f);
    }

    /// Applies `f` to each element of `self` and collect its result into a new row.
    #[inline(always)]
    pub fn map<E: Entity>(self, f: impl FnMut(<Self as MatIndex>::Item) -> E) -> Row<E, C> {
        let (_, n) = (Self::nrows(&self), Self::ncols(&self));
        let mut out = Row::<E>::with_capacity(n.unbound());
        let out_view = unsafe { row::from_raw_parts_mut::<'_, E, _>(out.as_ptr_mut(), n, 1) };
        let mut f = f;
        ZipEq::new(out_view, self).for_each(
            #[inline(always)]
            |Zip(mut out, item)| out.write(f(item)),
        );
        unsafe { out.set_ncols(n.unbound()) };
        out.into_shape(n)
    }

    /// Applies `f` to each element of `self` and collect its result into a new row.
    #[inline(always)]
    pub fn map_with_index<E: Entity>(
        self,
        f: impl FnMut(Idx<C>, <Self as MatIndex>::Item) -> E,
    ) -> Row<E, C> {
        let (_, n) = (Self::nrows(&self), Self::ncols(&self));
        let mut out = Row::<E>::with_capacity(n.unbound());
        let out_view = unsafe { row::from_raw_parts_mut::<'_, E, _>(out.as_ptr_mut(), n, 1) };
        let mut f = f;
        ZipEq::new(out_view, self).for_each_with_index(
            #[inline(always)]
            |j, Zip(mut out, item)| out.write(f(j, item)),
        );
        unsafe { out.set_ncols(n.unbound()) };
        out.into_shape(n)
    }
}

impl<
        R: Shape,
        M: MatIndex<
            LayoutTransform = VecLayoutTransform,
            Rows = R,
            Cols = (),
            Index = Idx<R>,
            Dyn: MatIndex<
                LayoutTransform = VecLayoutTransform,
                Item = M::Item,
                Slice = M::Slice,
                Rows = usize,
                Cols = (),
                Index = usize,
                Dyn = M::Dyn,
            >,
        >,
    > LastEq<R, (), M>
{
    /// Applies `f` to each element of `self`.
    #[inline(always)]
    pub fn for_each(self, f: impl FnMut(<Self as MatIndex>::Item)) {
        for_each_col(self, f);
    }

    /// Applies `f` to each element of `self`, while passing in the index of the current element.
    #[inline(always)]
    pub fn for_each_with_index(self, f: impl FnMut(Idx<R>, <Self as MatIndex>::Item)) {
        for_each_col_with_index(self, f);
    }

    /// Applies `f` to each element of `self` and collect its result into a new column.
    #[inline(always)]
    pub fn map<E: Entity>(self, f: impl FnMut(<Self as MatIndex>::Item) -> E) -> Col<E, R> {
        let (m, _) = (Self::nrows(&self), Self::ncols(&self));
        let mut out = Col::<E>::with_capacity(m.unbound());
        let out_view = unsafe { col::from_raw_parts_mut::<'_, E, _>(out.as_ptr_mut(), m, 1) };
        let mut f = f;
        ZipEq::new(out_view, self).for_each(
            #[inline(always)]
            |Zip(mut out, item)| out.write(f(item)),
        );
        unsafe { out.set_nrows(m.unbound()) };
        out.into_shape(m)
    }

    /// Applies `f` to each element of `self` and collect its result into a new column.
    #[inline(always)]
    pub fn map_with_index<E: Entity>(
        self,
        f: impl FnMut(Idx<R>, <Self as MatIndex>::Item) -> E,
    ) -> Col<E, R> {
        let (m, _) = (Self::nrows(&self), Self::ncols(&self));
        let mut out = Col::<E>::with_capacity(m.unbound());
        let out_view = unsafe { col::from_raw_parts_mut::<'_, E, _>(out.as_ptr_mut(), m, 1) };
        let mut f = f;
        ZipEq::new(out_view, self).for_each_with_index(
            #[inline(always)]
            |i, Zip(mut out, item)| out.write(f(i, item)),
        );
        unsafe { out.set_nrows(m.unbound()) };
        out.into_shape(m)
    }
}

impl<
        C: Shape,
        Head: MatIndex<
            Rows = (),
            Cols = C,
            Index = Idx<C>,
            LayoutTransform = VecLayoutTransform,
            Dyn: MatIndex<
                Item = Head::Item,
                Slice = Head::Slice,
                Rows = (),
                Cols = usize,
                Index = usize,
                LayoutTransform = VecLayoutTransform,
                Dyn = Head::Dyn,
            >,
        >,
        Tail: MatIndex<
            Rows = (),
            Cols = C,
            Index = Idx<C>,
            LayoutTransform = VecLayoutTransform,
            Dyn: MatIndex<
                Item = Tail::Item,
                Slice = Tail::Slice,
                Rows = (),
                Cols = usize,
                Index = usize,
                LayoutTransform = VecLayoutTransform,
                Dyn = Tail::Dyn,
            >,
        >,
    > ZipEq<(), C, Head, Tail>
{
    /// Applies `f` to each element of `self`.
    #[inline(always)]
    pub fn for_each(self, f: impl FnMut(<Self as MatIndex>::Item)) {
        for_each_row(self, f);
    }

    /// Applies `f` to each element of `self`, while passing in the index of the current element.
    #[inline(always)]
    pub fn for_each_with_index(self, f: impl FnMut(Idx<C>, <Self as MatIndex>::Item)) {
        for_each_row_with_index(self, f);
    }

    /// Applies `f` to each element of `self` and collect its result into a new row.
    #[inline(always)]
    pub fn map<E: Entity>(self, f: impl FnMut(<Self as MatIndex>::Item) -> E) -> Row<E, C> {
        let (_, n) = (Self::nrows(&self), Self::ncols(&self));
        let mut out = Row::<E>::with_capacity(n.unbound());
        let out_view = unsafe { row::from_raw_parts_mut::<'_, E, _>(out.as_ptr_mut(), n, 1) };
        let mut f = f;
        ZipEq::new(out_view, self).for_each(
            #[inline(always)]
            |Zip(mut out, item)| out.write(f(item)),
        );
        unsafe { out.set_ncols(n.unbound()) };
        out.into_shape(n)
    }
}

impl<
        R: Shape,
        Head: MatIndex<
            Rows = R,
            Cols = (),
            Index = Idx<R>,
            LayoutTransform = VecLayoutTransform,
            Dyn: MatIndex<
                Item = Head::Item,
                Slice = Head::Slice,
                Rows = usize,
                Cols = (),
                Index = usize,
                LayoutTransform = VecLayoutTransform,
                Dyn = Head::Dyn,
            >,
        >,
        Tail: MatIndex<
            Rows = R,
            Cols = (),
            Index = Idx<R>,
            LayoutTransform = VecLayoutTransform,
            Dyn: MatIndex<
                Item = Tail::Item,
                Slice = Tail::Slice,
                Rows = usize,
                Cols = (),
                Index = usize,
                LayoutTransform = VecLayoutTransform,
                Dyn = Tail::Dyn,
            >,
        >,
    > ZipEq<R, (), Head, Tail>
{
    /// Applies `f` to each element of `self`.
    #[inline(always)]
    pub fn for_each(self, f: impl FnMut(<Self as MatIndex>::Item)) {
        for_each_col(self, f);
    }

    /// Applies `f` to each element of `self`, while passing in the index of the current element.
    #[inline(always)]
    pub fn for_each_with_index(self, f: impl FnMut(Idx<R>, <Self as MatIndex>::Item)) {
        for_each_col_with_index(self, f);
    }

    /// Applies `f` to each element of `self` and collect its result into a new column.
    #[inline(always)]
    pub fn map<E: Entity>(self, f: impl FnMut(<Self as MatIndex>::Item) -> E) -> Col<E, R> {
        let (m, _) = (Self::nrows(&self), Self::ncols(&self));
        let mut out = Col::<E>::with_capacity(m.unbound());
        let out_view = unsafe { col::from_raw_parts_mut::<'_, E, _>(out.as_ptr_mut(), m, 1) };
        let mut f = f;
        ZipEq::new(out_view, self).for_each(
            #[inline(always)]
            |Zip(mut out, item)| out.write(f(item)),
        );
        unsafe { out.set_nrows(m.unbound()) };
        out.into_shape(m)
    }
}

impl<
        R: Shape,
        C: Shape,
        Head: MatIndex<
            LayoutTransform = MatLayoutTransform,
            Rows = R,
            Cols = C,
            Index = (Idx<R>, Idx<C>),
            Dyn: MatIndex<
                LayoutTransform = MatLayoutTransform,
                Item = Head::Item,
                Slice = Head::Slice,
                Rows = usize,
                Cols = usize,
                Index = (usize, usize),
                Dyn = Head::Dyn,
            >,
        >,
        Tail: MatIndex<
            LayoutTransform = MatLayoutTransform,
            Rows = R,
            Cols = C,
            Index = (Idx<R>, Idx<C>),
            Dyn: MatIndex<
                LayoutTransform = MatLayoutTransform,
                Item = Tail::Item,
                Slice = Tail::Slice,
                Rows = usize,
                Cols = usize,
                Index = (usize, usize),
                Dyn = Tail::Dyn,
            >,
        >,
    > ZipEq<R, C, Head, Tail>
{
    /// Applies `f` to each element of `self`.
    #[inline(always)]
    pub fn for_each(self, f: impl FnMut(<Self as MatIndex>::Item)) {
        for_each_mat(self, f);
    }

    /// Applies `f` to each element of `self`, while passing the indices of the position of the
    /// current element.
    #[inline(always)]
    pub fn for_each_with_index(self, f: impl FnMut(Idx<R>, Idx<C>, <Self as MatIndex>::Item)) {
        for_each_mat_with_index(self, f);
    }

    /// Applies `f` to each element of the lower triangular half of `self`, while passing the
    /// indices of the position of the current element.
    ///
    /// `diag` specifies whether the diagonal should be included or excluded.
    #[inline(always)]
    pub fn for_each_triangular_lower_with_index(
        self,
        diag: Diag,
        f: impl FnMut(Idx<R>, Idx<C>, <Self as MatIndex>::Item),
    ) {
        for_each_mat_triangular_lower_with_index(self, diag, f);
    }

    /// Applies `f` to each element of the upper triangular half of `self`, while passing the
    /// indices of the position of the current element.
    ///
    /// `diag` specifies whether the diagonal should be included or excluded.
    #[inline(always)]
    pub fn for_each_triangular_upper_with_index(
        self,
        diag: Diag,
        f: impl FnMut(Idx<R>, Idx<C>, <Self as MatIndex>::Item),
    ) {
        for_each_mat_triangular_upper_with_index(self, diag, f);
    }

    /// Applies `f` to each element of the lower triangular half of `self`.
    ///
    /// `diag` specifies whether the diagonal should be included or excluded.
    #[inline(always)]
    pub fn for_each_triangular_lower(self, diag: Diag, f: impl FnMut(<Self as MatIndex>::Item)) {
        for_each_mat_triangular_lower(self, diag, false, f);
    }

    /// Applies `f` to each element of the upper triangular half of `self`.
    ///
    /// `diag` specifies whether the diagonal should be included or excluded.
    #[inline(always)]
    pub fn for_each_triangular_upper(self, diag: Diag, f: impl FnMut(<Self as MatIndex>::Item)) {
        for_each_mat_triangular_lower(self, diag, true, f);
    }

    /// Applies `f` to each element of `self` and collect its result into a new matrix.
    #[inline(always)]
    pub fn map<E: Entity>(self, f: impl FnMut(<Self as MatIndex>::Item) -> E) -> Mat<E, R, C> {
        let (m, n) = (Self::nrows(&self), Self::ncols(&self));
        let mut out = Mat::<E>::with_capacity(m.unbound(), n.unbound());
        let rs = 1;
        let cs = out.col_stride();
        let out_view =
            unsafe { mat::from_raw_parts_mut::<'_, E, _, _>(out.as_ptr_mut(), m, n, rs, cs) };
        let mut f = f;
        ZipEq::new(out_view, self).for_each(
            #[inline(always)]
            |Zip(mut out, item)| out.write(f(item)),
        );
        unsafe { out.set_dims(m.unbound(), n.unbound()) };
        out.into_shape(m, n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{assert, unzipped, zipped};

    #[test]
    fn test_zip() {
        for (m, n) in [(2, 2), (4, 2), (2, 4)] {
            for rev_dst in [false, true] {
                for rev_src in [false, true] {
                    for transpose_dst in [false, true] {
                        for transpose_src in [false, true] {
                            for diag in [Diag::Include, Diag::Skip] {
                                let mut dst = Mat::from_fn(
                                    if transpose_dst { n } else { m },
                                    if transpose_dst { m } else { n },
                                    |_, _| f64::faer_zero(),
                                );
                                let src = Mat::from_fn(
                                    if transpose_src { n } else { m },
                                    if transpose_src { m } else { n },
                                    |_, _| f64::faer_one(),
                                );

                                let mut target = Mat::from_fn(m, n, |_, _| f64::faer_zero());
                                let target_src = Mat::from_fn(m, n, |_, _| f64::faer_one());

                                zipped_rw!(target.as_mut(), target_src.as_ref())
                                    .for_each_triangular_lower(diag, |unzipped!(mut dst, src)| {
                                        dst.write(src.read())
                                    });

                                let mut dst = dst.as_mut();
                                let mut src = src.as_ref();

                                if transpose_dst {
                                    dst = dst.transpose_mut();
                                }
                                if rev_dst {
                                    dst = dst.reverse_rows_mut();
                                }

                                if transpose_src {
                                    src = src.transpose();
                                }
                                if rev_src {
                                    src = src.reverse_rows();
                                }

                                zipped_rw!(dst.rb_mut(), src)
                                    .for_each_triangular_lower(diag, |unzipped!(mut dst, src)| {
                                        dst.write(src.read())
                                    });

                                assert!(dst.rb() == target.as_ref());
                            }
                        }
                    }
                }
            }
        }

        {
            let m = 3;
            for rev_dst in [false, true] {
                for rev_src in [false, true] {
                    let mut dst = Col::<f64>::zeros(m);
                    let src = Col::from_fn(m, |i| (i + 1) as f64);

                    let mut target = Col::<f64>::zeros(m);
                    let target_src =
                        Col::from_fn(m, |i| if rev_src { m - i } else { i + 1 } as f64);

                    zipped_rw!(target.as_mut(), target_src.as_ref())
                        .for_each(|unzipped!(mut dst, src)| dst.write(src.read()));

                    let mut dst = dst.as_mut();
                    let mut src = src.as_ref();

                    if rev_dst {
                        dst = dst.reverse_rows_mut();
                    }
                    if rev_src {
                        src = src.reverse_rows();
                    }

                    zipped_rw!(dst.rb_mut(), src)
                        .for_each(|unzipped!(mut dst, src)| dst.write(src.read()));

                    assert!(dst.rb() == target.as_ref());
                }
            }
        }

        {
            let m = 3;
            for rev_dst in [false, true] {
                for rev_src in [false, true] {
                    let mut dst = Row::<f64>::zeros(m);
                    let src = Row::from_fn(m, |i| (i + 1) as f64);

                    let mut target = Row::<f64>::zeros(m);
                    let target_src =
                        Row::from_fn(m, |i| if rev_src { m - i } else { i + 1 } as f64);

                    zipped_rw!(target.as_mut(), target_src.as_ref())
                        .for_each(|unzipped!(mut dst, src)| dst.write(src.read()));

                    let mut dst = dst.as_mut();
                    let mut src = src.as_ref();

                    if rev_dst {
                        dst = dst.reverse_cols_mut();
                    }
                    if rev_src {
                        src = src.reverse_cols();
                    }

                    zipped_rw!(&mut dst, src)
                        .for_each(|unzipped!(mut dst, src)| dst.write(src.read()));

                    assert!(dst.rb() == target.as_ref());
                }
            }
        }
    }

    #[test]
    fn test_zip_with_index() {
        let m = mat![
            [0.0, 1.0, 2.0],
            [3.0, 4.0, 5.0],
            [6.0, 7.0, 8.0],
            [9.0, 10.0, 11.0f64]
        ];
        let nan = f64::NAN;
        let m_non_contiguous = mat![
            [0.0, 1.0, 2.0],
            [nan, nan, nan],
            [3.0, 4.0, 5.0],
            [nan, nan, nan],
            [6.0, 7.0, 8.0],
            [nan, nan, nan],
            [9.0, 10.0, 11.0],
            [nan, nan, nan],
        ];

        let m_non_contiguous = unsafe {
            crate::mat::from_raw_parts(
                m_non_contiguous.as_ptr(),
                4,
                3,
                2,
                m_non_contiguous.col_stride(),
            )
        };

        for m in [m.as_ref(), m_non_contiguous] {
            {
                let m = m.as_ref();
                zipped_rw!(m).for_each_with_index(|i, j, unzipped!(val)| {
                    assert!(*val == (j + i * 3) as f64);
                });

                for diag in [Diag::Include, Diag::Skip] {
                    zipped_rw!(m).for_each_triangular_lower_with_index(
                        diag,
                        |i, j, unzipped!(val)| {
                            if diag == Diag::Include {
                                assert!(i >= j);
                            } else {
                                assert!(i > j);
                            }
                            assert!(*val == (j + i * 3) as f64);
                        },
                    );
                    zipped_rw!(m).for_each_triangular_upper_with_index(
                        diag,
                        |i, j, unzipped!(val)| {
                            if diag == Diag::Include {
                                assert!(i <= j);
                            } else {
                                assert!(i < j);
                            }
                            assert!(*val == (j + i * 3) as f64);
                        },
                    );
                }
            }
            {
                let m = m.as_ref().reverse_rows();
                zipped_rw!(m).for_each_with_index(|i, j, unzipped!(val)| {
                    assert!(*val == (j + (3 - i) * 3) as f64);
                });

                for diag in [Diag::Include, Diag::Skip] {
                    zipped_rw!(m).for_each_triangular_lower_with_index(
                        diag,
                        |i, j, unzipped!(val)| {
                            if diag == Diag::Include {
                                assert!(i >= j);
                            } else {
                                assert!(i > j);
                            }

                            assert!(*val == (j + (3 - i) * 3) as f64);
                        },
                    );
                    zipped_rw!(m).for_each_triangular_upper_with_index(
                        diag,
                        |i, j, unzipped!(val)| {
                            if diag == Diag::Include {
                                assert!(i <= j);
                            } else {
                                assert!(i < j);
                            }

                            assert!(*val == (j + (3 - i) * 3) as f64);
                        },
                    );
                }
            }
            {
                let m = m.as_ref().transpose();
                zipped_rw!(m).for_each_with_index(|i, j, unzipped!(val)| {
                    assert!(*val == (i + j * 3) as f64);
                });

                for diag in [Diag::Include, Diag::Skip] {
                    zipped_rw!(m).for_each_triangular_lower_with_index(
                        diag,
                        |i, j, unzipped!(val)| {
                            if diag == Diag::Include {
                                assert!(i >= j);
                            } else {
                                assert!(i > j);
                            }

                            assert!(*val == (i + j * 3) as f64);
                        },
                    );
                    zipped_rw!(m).for_each_triangular_upper_with_index(
                        diag,
                        |i, j, unzipped!(val)| {
                            if diag == Diag::Include {
                                assert!(i <= j);
                            } else {
                                assert!(i < j);
                            }

                            assert!(*val == (i + j * 3) as f64);
                        },
                    );
                }
            }
            {
                let m = m.as_ref().reverse_rows().transpose();
                zipped_rw!(m).for_each_with_index(|i, j, unzipped!(val)| {
                    assert!(*val == (i + (3 - j) * 3) as f64);
                });

                for diag in [Diag::Include, Diag::Skip] {
                    zipped_rw!(m).for_each_triangular_lower_with_index(
                        diag,
                        |i, j, unzipped!(val)| {
                            if diag == Diag::Include {
                                assert!(i >= j);
                            } else {
                                assert!(i > j);
                            }

                            assert!(*val == (i + (3 - j) * 3) as f64);
                        },
                    );
                    zipped_rw!(m).for_each_triangular_upper_with_index(
                        diag,
                        |i, j, unzipped!(val)| {
                            if diag == Diag::Include {
                                assert!(i <= j);
                            } else {
                                assert!(i < j);
                            }

                            assert!(*val == (i + (3 - j) * 3) as f64);
                        },
                    );
                }
            }
        }

        for m in [m.as_ref().col(0), m_non_contiguous.col(0)] {
            {
                zipped_rw!(m).for_each_with_index(|i, unzipped!(val)| {
                    assert!(*val == (i * 3) as f64);
                });
                zipped_rw!(m).for_each_with_index(|i, unzipped!(val)| {
                    assert!(*val == (i * 3) as f64);
                });
            }

            {
                let m = m.reverse_rows();
                zipped_rw!(m).for_each_with_index(|i, unzipped!(val)| {
                    assert!(*val == ((3 - i) * 3) as f64);
                });
            }
        }

        for m in [m.as_ref().row(0), m_non_contiguous.row(0)] {
            {
                zipped_rw!(m).for_each_with_index(|j, unzipped!(val)| {
                    assert!(*val == j as f64);
                });

                zipped!(m).for_each_with_index(|j, unzipped!(val)| {
                    assert!(*val == j as f64);
                });
            }

            {
                let m = m.reverse_cols();
                zipped_rw!(m).for_each_with_index(|j, unzipped!(val)| {
                    assert!(*val == (2 - j) as f64);
                });
            }
        }
    }
}
