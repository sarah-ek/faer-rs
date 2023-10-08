// implementation inspired by https://gitlab.com/hodge_star/catamari

use super::*;
use crate::{
    amd::Control,
    ghost::{Array, Idx, MaybeIdx},
};
use assert2::{assert, debug_assert};
use core::cell::Cell;
use dyn_stack::PodStack;
use faer_core::{temp_mat_req, temp_mat_uninit, zipped, MatMut, MatRef, Parallelism};

#[derive(Copy, Clone)]
pub enum Ordering<'a, I> {
    Identity,
    Custom(&'a [I]),
    Algorithm(
        &'a dyn Fn(
            &mut [I],                       // perm
            &mut [I],                       // perm_inv
            SymbolicSparseColMatRef<'_, I>, // A
            PodStack<'_>,
        ) -> Result<(), FaerSparseError>,
    ),
}

// workspace: I*(n)
pub fn ghost_prefactorize_symbolic<'n, 'out, I: Index>(
    etree: &'out mut Array<'n, I>,
    col_counts: &mut Array<'n, I>,
    A: ghost::SymbolicSparseColMatRef<'n, 'n, '_, I>,
    stack: PodStack<'_>,
) -> &'out mut Array<'n, MaybeIdx<'n, I>> {
    let N = A.ncols();
    let etree: &mut [I] = etree;
    let (mut visited, _) = stack.make_raw::<I>(*N);
    let etree = Array::from_mut(ghost::fill_none(etree, N), N);
    let visited = Array::from_mut(&mut visited, N);

    for j in N.indices() {
        let j_ = j.truncate::<I>();
        visited[j] = *j_;
        col_counts[j] = I::truncate(1);

        for mut i in A.row_indices_of_col(j) {
            if i < j {
                loop {
                    if visited[i] == *j_ {
                        break;
                    }

                    let next_i = if let Some(parent) = etree[i].idx() {
                        parent.zx()
                    } else {
                        etree[i] = MaybeIdx::from_index(j_);
                        j
                    };

                    col_counts[i].incr();
                    visited[i] = *j_;
                    i = next_i;
                }
            }
        }
    }

    etree
}

#[derive(Copy, Clone, Debug)]
pub struct CholeskySymbolicSupernodalParams<'a> {
    pub relax: Option<&'a [(usize, f64)]>,
}

impl Default for CholeskySymbolicSupernodalParams<'_> {
    #[inline]
    fn default() -> Self {
        Self {
            relax: Some(&[(4, 1.0), (16, 0.8), (48, 0.1), (usize::MAX, 0.05)]),
        }
    }
}

fn ereach<'n, 'a, I: Index>(
    stack: &'a mut Array<'n, I>,
    A: ghost::SymbolicSparseColMatRef<'n, 'n, '_, I>,
    etree: &Array<'n, MaybeIdx<'n, I>>,
    k: Idx<'n>,
    visited: &mut Array<'n, I>,
) -> &'a [Idx<'n, I>] {
    let N = A.ncols();

    // invariant: stack[top..] elements are less than or equal to k
    let mut top = *N;
    let k_: I = *k.truncate();
    visited[k] = k_;
    for mut i in A.row_indices_of_col(k) {
        // (1): after this, we know i < k
        if i >= k {
            continue;
        }
        // invariant: stack[..len] elements are less than or equal to k
        let mut len = 0usize;
        loop {
            if visited[i] == k_ {
                break;
            }

            // inserted element is i < k, see (1)
            let pushed: Idx<'n, I> = i.truncate::<I>();
            stack[N.check(len)] = *pushed;
            // len is incremented, maintaining the invariant
            len += 1;

            visited[i] = k_;
            i = N.check(etree[i].zx());
        }

        // because stack[..len] elements are less than or equal to k
        // stack[top - len..] elements are now less than or equal to k
        stack.copy_within(..len, top - len);
        // top is decremented by len, maintaining the invariant
        top -= len;
    }

    let stack = &(**stack)[top..];

    // SAFETY: stack[top..] elements are < k < N
    unsafe { Idx::slice_ref_unchecked(stack, N) }
}

fn ereach_super<'n, 'nsuper, I: Index>(
    A: ghost::SymbolicSparseColMatRef<'n, 'n, '_, I>,
    super_etree: &Array<'nsuper, MaybeIdx<'nsuper, I>>,
    index_to_super: &Array<'n, Idx<'nsuper, I>>,
    current_row_positions: &mut Array<'nsuper, I>,
    row_indices: &mut [Idx<'n, I>],
    k: Idx<'n>,
    visited: &mut Array<'nsuper, I>,
) {
    let k_: I = *k.truncate();
    visited[index_to_super[k].zx()] = k_;
    for i in A.row_indices_of_col(k) {
        if i >= k {
            continue;
        }
        let mut supernode_i = index_to_super[i].zx();
        loop {
            if visited[supernode_i] == k_ {
                break;
            }

            row_indices[current_row_positions[supernode_i].zx()] = k.truncate();
            current_row_positions[supernode_i].incr();

            visited[supernode_i] = k_;
            supernode_i = super_etree[supernode_i].sx().idx().unwrap();
        }
    }
}

pub fn factorize_simplicial_symbolic_req<I: Index>(n: usize) -> Result<StackReq, SizeOverflow> {
    let n_req = StackReq::try_new::<I>(n)?;
    StackReq::try_all_of([n_req, n_req, n_req])
}

pub fn ghost_factorize_simplicial_symbolic<'n, I: Index>(
    A: ghost::SymbolicSparseColMatRef<'n, 'n, '_, I>,
    etree: &Array<'n, MaybeIdx<'n, I>>,
    col_counts: &Array<'n, I>,
    stack: PodStack<'_>,
) -> Result<SymbolicSimplicialCholesky<I>, FaerSparseError> {
    let N = A.ncols();
    let n = *N;

    let mut L_col_ptrs = try_zeroed::<I>(n + 1)?;
    for (&count, [p, p_next]) in zip(
        &**col_counts,
        windows2(Cell::as_slice_of_cells(Cell::from_mut(&mut L_col_ptrs))),
    ) {
        p_next.set(p.get() + count);
    }
    let l_nnz = L_col_ptrs[n].zx();
    let mut L_row_ind = try_zeroed::<I>(l_nnz)?;

    ghost::with_size(
        l_nnz,
        #[inline(always)]
        move |L_NNZ| {
            let (mut current_row_index, stack) = stack.make_raw::<I>(n);
            let (mut ereach_stack, stack) = stack.make_raw::<I>(n);
            let (mut marked, _) = stack.make_raw::<I>(n);

            let ereach_stack = Array::from_mut(&mut ereach_stack, N);
            let etree = Array::from_ref(etree, N);
            let visited = Array::from_mut(&mut marked, N);

            mem::fill_none(visited);
            let L_row_indices = Array::from_mut(&mut L_row_ind, L_NNZ);
            let L_col_ptrs_start =
                Array::from_ref(Idx::slice_ref_checked(&L_col_ptrs[..n], L_NNZ), N);
            let current_row_index = Array::from_mut(
                ghost::copy_slice(&mut current_row_index, L_col_ptrs_start),
                N,
            );

            for k in N.indices() {
                let reach = ereach(ereach_stack, A, etree, k, visited);
                for &j in reach {
                    let j = j.zx();
                    let cj = &mut current_row_index[j];
                    let row_idx = L_NNZ.check(*cj.zx() + 1);
                    *cj = row_idx.truncate();
                    L_row_indices[row_idx] = *k.truncate();
                }
                let k_start = L_col_ptrs_start[k].zx();
                L_row_indices[k_start] = *k.truncate();
            }

            let etree = try_collect(MaybeIdx::as_inner_slice_ref(etree).iter().copied())?;

            Ok(SymbolicSimplicialCholesky {
                dimension: n,
                col_ptrs: L_col_ptrs,
                row_indices: L_row_ind,
                etree,
            })
        },
    )
}

pub fn factorize_simplicial_numeric_ldlt<I: Index, E: ComplexField>(
    L_values: SliceGroupMut<'_, E>,
    A: SparseColMatRef<'_, I, E>,
    symbolic: &SymbolicSimplicialCholesky<I>,
    stack: PodStack<'_>,
) {
    let n = A.ncols();
    let L_row_indices = &*symbolic.row_indices;
    let L_col_ptrs = &*symbolic.col_ptrs;
    let etree = &*symbolic.etree;

    assert!(L_values.rb().len() == L_row_indices.len());
    assert!(L_col_ptrs.len() == n + 1);
    let l_nnz = L_col_ptrs[n].zx();

    ghost::with_size(
        n,
        #[inline(always)]
        |N| {
            let etree = Array::from_ref(MaybeIdx::slice_ref_checked(etree, N), N);
            let A = ghost::SparseColMatRef::new(A, N, N);

            ghost::with_size(
                l_nnz,
                #[inline(always)]
                move |L_NNZ| {
                    let (mut x, stack) = crate::make_raw::<E>(n, stack);
                    let (mut current_row_index, stack) = stack.make_raw::<I>(n);
                    let (mut ereach_stack, stack) = stack.make_raw::<I>(n);
                    let (mut marked, _) = stack.make_raw::<I>(n);

                    let ereach_stack = Array::from_mut(&mut ereach_stack, N);
                    let etree = Array::from_ref(etree, N);
                    let visited = Array::from_mut(&mut marked, N);
                    let mut x = ghost::ArrayGroupMut::new(
                        SliceGroupMut::new(E::map(E::as_mut(&mut x), |x| &mut **x)),
                        N,
                    );

                    x.rb_mut().into_slice().fill_zero();
                    mem::fill_none(visited);

                    let mut L_values = ghost::ArrayGroupMut::new(L_values, L_NNZ);
                    let L_row_indices = Array::from_ref(L_row_indices, L_NNZ);

                    let L_col_ptrs_start =
                        Array::from_ref(Idx::slice_ref_checked(&L_col_ptrs[..n], L_NNZ), N);

                    let current_row_index = Array::from_mut(
                        ghost::copy_slice(&mut current_row_index, L_col_ptrs_start),
                        N,
                    );

                    for k in N.indices() {
                        let reach = ereach(ereach_stack, A.symbolic(), etree, k, visited);

                        for (i, aik) in zip(A.row_indices_of_col(k), A.values_of_col(k).into_iter())
                        {
                            x.write(i, aik.read().conj());
                        }

                        let mut d = x.read(k).real();
                        x.write(k, E::zero());

                        for &j in reach {
                            let j = j.zx();

                            let j_start = L_col_ptrs_start[j].zx();
                            let cj = &mut current_row_index[j];
                            let row_idx = L_NNZ.check(*cj.zx() + 1);
                            *cj = row_idx.truncate();

                            let xj = x.read(j);
                            x.write(j, E::zero());

                            let dj = L_values.read(j_start).real();
                            let lkj = xj.scale_real(dj.inv());

                            let range = j_start.next()..row_idx.to_inclusive();
                            for (i, lij) in zip(
                                &L_row_indices[range.clone()],
                                L_values.rb().subslice(range).into_iter(),
                            ) {
                                let i = N.check(i.zx());
                                let mut xi = x.read(i);
                                let prod = lij.read().conj().mul(xj);
                                xi = xi.sub(prod);
                                x.write(i, xi);
                            }

                            d = d.sub(lkj.mul(xj.conj()).real());

                            L_values.write(row_idx, lkj);
                        }

                        let k_start = L_col_ptrs_start[k].zx();
                        L_values.write(k_start, E::from_real(d));
                    }
                },
            )
        },
    )
}

pub fn factorize_simplicial_numeric_with_row_indices<I: Index, E: ComplexField>(
    L_values: SliceGroupMut<'_, E>,
    L_row_indices: &mut [I],
    L_col_ptrs: &[I],

    etree: &[I],
    A: SparseColMatRef<'_, I, E>,

    stack: PodStack<'_>,
) {
    let n = A.ncols();
    assert!(L_values.rb().len() == L_row_indices.len());
    assert!(L_col_ptrs.len() == n + 1);
    let l_nnz = L_col_ptrs[n].zx();

    ghost::with_size(
        n,
        #[inline(always)]
        |N| {
            let etree = Array::from_ref(MaybeIdx::slice_ref_checked(etree, N), N);
            let A = ghost::SparseColMatRef::new(A, N, N);

            ghost::with_size(
                l_nnz,
                #[inline(always)]
                move |L_NNZ| {
                    let (mut x, stack) = crate::make_raw::<E>(n, stack);
                    let (mut current_row_index, stack) = stack.make_raw::<I>(n);
                    let (mut ereach_stack, stack) = stack.make_raw::<I>(n);
                    let (mut marked, _) = stack.make_raw::<I>(n);

                    let ereach_stack = Array::from_mut(&mut ereach_stack, N);
                    let etree = Array::from_ref(etree, N);
                    let visited = Array::from_mut(&mut marked, N);
                    let mut x = ghost::ArrayGroupMut::new(
                        SliceGroupMut::new(E::map(E::as_mut(&mut x), |x| &mut **x)),
                        N,
                    );

                    x.rb_mut().into_slice().fill_zero();
                    mem::fill_none(visited);

                    let mut L_values = ghost::ArrayGroupMut::new(L_values, L_NNZ);
                    let L_row_indices = Array::from_mut(L_row_indices, L_NNZ);

                    let L_col_ptrs_start =
                        Array::from_ref(Idx::slice_ref_checked(&L_col_ptrs[..n], L_NNZ), N);

                    let current_row_index = Array::from_mut(
                        ghost::copy_slice(&mut current_row_index, L_col_ptrs_start),
                        N,
                    );

                    for k in N.indices() {
                        let reach = ereach(ereach_stack, A.symbolic(), etree, k, visited);

                        for (i, aik) in zip(A.row_indices_of_col(k), A.values_of_col(k).into_iter())
                        {
                            x.write(i, aik.read().conj());
                        }

                        let mut d = x.read(k).real();
                        x.write(k, E::zero());

                        for &j in reach {
                            let j = j.zx();

                            let j_start = L_col_ptrs_start[j].zx();
                            let cj = &mut current_row_index[j];
                            let row_idx = L_NNZ.check(*cj.zx() + 1);
                            *cj = row_idx.truncate();

                            let xj = x.read(j);
                            x.write(j, E::zero());

                            let dj = L_values.read(j_start).real();
                            let lkj = xj.scale_real(dj.inv());

                            let range = j_start.next()..row_idx.to_inclusive();
                            for (i, lij) in zip(
                                &L_row_indices[range.clone()],
                                L_values.rb().subslice(range).into_iter(),
                            ) {
                                let i = N.check(i.zx());
                                let mut xi = x.read(i);
                                let prod = lij.read().conj().mul(xj);
                                xi = xi.sub(prod);
                                x.write(i, xi);
                            }

                            d = d.sub(lkj.mul(xj.conj()).real());

                            L_row_indices[row_idx] = *k.truncate();
                            L_values.write(row_idx, lkj);
                        }

                        let k_start = L_col_ptrs_start[k].zx();
                        L_row_indices[k_start] = *k.truncate();
                        L_values.write(k_start, E::from_real(d));
                    }
                },
            )
        },
    )
}

#[derive(Debug, Copy, Clone)]
pub struct ComputationModel {
    pub ldl: [f64; 4],
    pub triangular_solve: [f64; 6],
    pub matmul: [f64; 6],
    pub assembly: [f64; 4],
}

impl ComputationModel {
    #[allow(clippy::excessive_precision)]
    pub const OPENBLAS_I7_1185G7: Self = ComputationModel {
        ldl: [
            3.527141723946874224e-07,
            -5.382557351808083451e-08,
            4.677984682984275924e-09,
            7.384424667338682676e-12,
        ],
        triangular_solve: [
            1.101115592925888909e-06,
            6.936563076265144074e-07,
            -1.827661167503034051e-09,
            1.959826916788009885e-09,
            1.079857543323972179e-09,
            2.963338652996178598e-11,
        ],
        matmul: [
            6.14190596709488416e-07,
            -4.489948374364910256e-09,
            5.943145978912038475e-10,
            -1.201283634136652872e-08,
            1.266858215451465993e-09,
            2.624001993284897048e-11,
        ],
        assembly: [
            3.069607518266660019e-07,
            3.763778311956422235e-08,
            1.991443920635728855e-07,
            3.788938150548870089e-09,
        ],
    };

    #[inline]
    pub fn ldl_estimate(&self, n: f64) -> f64 {
        let p = self.ldl;
        p[0] + n * (p[1] + n * (p[2] + n * p[3]))
    }

    #[inline]
    pub fn triangular_solve_estimate(&self, n: f64, k: f64) -> f64 {
        let p = self.triangular_solve;
        p[0] + n * (p[1] + n * p[2]) + k * (p[3] + n * (p[4] + n * p[5]))
    }

    #[inline]
    pub fn matmul_estimate(&self, m: f64, n: f64, k: f64) -> f64 {
        let p = self.matmul;
        p[0] + (m + n) * p[1] + (m * n) * p[2] + k * (p[3] + (m + n) * p[4] + (m * n) * p[5])
    }

    #[inline]
    pub fn assembly_estimate(&self, br: f64, bc: f64) -> f64 {
        let p = self.assembly;
        p[0] + br * p[1] + bc * p[2] + br * bc * p[3]
    }
}

#[derive(Debug)]
pub struct SymbolicSupernodalCholesky<I> {
    dimension: usize,
    supernode_postorder: Vec<I>,
    supernode_postorder_inv: Vec<I>,
    descendent_count: Vec<I>,

    supernode_begin: Vec<I>,
    col_ptrs_for_row_indices: Vec<I>,
    col_ptrs_for_values: Vec<I>,
    row_indices: Vec<I>,
}

#[derive(Debug)]
pub struct SymbolicSimplicialCholesky<I> {
    dimension: usize,
    col_ptrs: Vec<I>,
    row_indices: Vec<I>,
    etree: Vec<I>,
}

#[derive(Debug)]
pub enum SymbolicCholeskyRaw<I> {
    Simplicial(SymbolicSimplicialCholesky<I>),
    Supernodal(SymbolicSupernodalCholesky<I>),
}

#[derive(Debug)]
pub struct SymbolicCholesky<I> {
    raw: SymbolicCholeskyRaw<I>,
    perm_fwd: Vec<I>,
    perm_inv: Vec<I>,
    A_nnz: usize,
}

impl<I: Index> SymbolicCholesky<I> {
    #[inline]
    pub fn nrows(&self) -> usize {
        match &self.raw {
            &SymbolicCholeskyRaw::Simplicial(ref this) => this.nrows(),
            &SymbolicCholeskyRaw::Supernodal(ref this) => this.nrows(),
        }
    }

    #[inline]
    pub fn ncols(&self) -> usize {
        self.nrows()
    }

    #[inline]
    pub fn raw(&self) -> &SymbolicCholeskyRaw<I> {
        &self.raw
    }

    #[inline]
    pub fn perm(&self) -> PermutationRef<'_, I> {
        unsafe { PermutationRef::new_unchecked(&self.perm_fwd, &self.perm_inv) }
    }

    #[inline]
    pub fn len_values(&self) -> usize {
        match &self.raw {
            &SymbolicCholeskyRaw::Simplicial(ref this) => this.len_values(),
            &SymbolicCholeskyRaw::Supernodal(ref this) => this.len_values(),
        }
    }

    #[inline]
    pub fn factorize_numeric_ldlt_req<E: Entity>(
        &self,
        side: Side,
        parallelism: Parallelism,
    ) -> Result<StackReq, SizeOverflow> {
        let n = self.nrows();
        let A_nnz = self.A_nnz;

        let n_req = StackReq::try_new::<I>(n)?;
        let A_req = StackReq::try_all_of([
            make_raw_req::<E>(A_nnz)?,
            StackReq::try_new::<I>(n + 1)?,
            StackReq::try_new::<I>(A_nnz)?,
        ])?;
        let A_req2 = if side == Side::Lower {
            A_req
        } else {
            StackReq::empty()
        };
        let permute_req = n_req;

        match &self.raw {
            &SymbolicCholeskyRaw::Simplicial(_) => {
                let simplicial_req = factorize_simplicial_numeric_ldlt_req::<I, E>(n)?;
                StackReq::try_all_of([
                    A_req2,
                    A_req,
                    StackReq::try_or(permute_req, simplicial_req)?,
                ])
            }
            &SymbolicCholeskyRaw::Supernodal(ref this) => {
                let transpose_req = n_req;
                let supernodal_req =
                    factorize_supernodal_numeric_ldlt_req::<I, E>(this, parallelism)?;

                StackReq::try_all_of([
                    A_req2,
                    A_req,
                    A_req,
                    StackReq::try_or(transpose_req, supernodal_req)?,
                ])
            }
        }
    }

    #[inline]
    pub fn factorize_numeric_ldlt<E: ComplexField>(
        &self,
        L_values: SliceGroupMut<'_, E>,
        A: SparseColMatRef<'_, I, E>,
        side: Side,
        parallelism: Parallelism,
        stack: PodStack<'_>,
    ) {
        assert!(A.nrows() == A.ncols());
        let n = A.nrows();
        let lower = (side == Side::Lower) as usize;

        ghost::with_size(n, |N| {
            let A_nnz = self.A_nnz;
            let A = ghost::SparseColMatRef::new(A, N, N);

            let perm = ghost::PermutationRef::new(self.perm(), N);

            let (mut new_values, stack) = crate::make_raw::<E>(lower * (A_nnz), stack);
            let (mut new_col_ptr, stack) = stack.make_raw::<I>(lower * (n + 1));
            let (mut new_row_ind, mut stack) = stack.make_raw::<I>(lower * (A_nnz));

            let A = if side == Side::Lower {
                let new_values =
                    SliceGroupMut::<'_, E>::new(E::map(E::as_mut(&mut new_values), |val| {
                        &mut **val
                    }));
                ghost_transpose(
                    &mut new_col_ptr,
                    &mut new_row_ind,
                    new_values,
                    A,
                    stack.rb_mut(),
                )
            } else {
                A
            };

            let (mut new_values, stack) = crate::make_raw::<E>(A_nnz, stack);
            let (mut new_col_ptr, stack) = stack.make_raw::<I>(n + 1);
            let (mut new_row_ind, mut stack) = stack.make_raw::<I>(A_nnz);
            let mut new_values =
                SliceGroupMut::<'_, E>::new(E::map(E::as_mut(&mut new_values), |val| &mut **val));

            let A = ghost_permute_symmetric(
                new_values.rb_mut(),
                &mut new_col_ptr,
                &mut new_row_ind,
                A,
                perm,
                stack.rb_mut(),
            );

            match &self.raw {
                &SymbolicCholeskyRaw::Simplicial(ref this) => {
                    factorize_simplicial_numeric_ldlt(L_values, *A, this, stack);
                }
                &SymbolicCholeskyRaw::Supernodal(ref this) => {
                    let (mut new_values, stack) = crate::make_raw::<E>(A_nnz, stack);
                    let (mut new_col_ptr, stack) = stack.make_raw::<I>(n + 1);
                    let (mut new_row_ind, mut stack) = stack.make_raw::<I>(A_nnz);
                    let mut new_values =
                        SliceGroupMut::<'_, E>::new(E::map(E::as_mut(&mut new_values), |val| {
                            &mut **val
                        }));
                    let A = ghost_transpose(
                        &mut new_col_ptr,
                        &mut new_row_ind,
                        new_values.rb_mut(),
                        A,
                        stack.rb_mut(),
                    );
                    factorize_supernodal_numeric_ldlt(L_values, *A, this, parallelism, stack);
                }
            }
        });
    }
}

#[derive(Debug)]
pub struct SupernodalLdltRef<'a, I, E: Entity> {
    symbolic: &'a SymbolicSupernodalCholesky<I>,
    values: SliceGroup<'a, E>,
}
#[derive(Debug)]
pub struct SimplicialLdltRef<'a, I, E: Entity> {
    symbolic: &'a SymbolicSimplicialCholesky<I>,
    values: SliceGroup<'a, E>,
}

#[derive(Debug)]
pub enum LdltRef<'a, I, E: Entity> {
    Simplicial(SimplicialLdltRef<'a, I, E>),
    Supernodal(SupernodalLdltRef<'a, I, E>),
}

impl_copy!(<'a><I, E: Entity><SupernodalLdltRef<'a, I, E>>);
impl_copy!(<'a><I, E: Entity><SimplicialLdltRef<'a, I, E>>);
impl_copy!(<'a><I, E: Entity><LdltRef<'a, I, E>>);

impl<'a, I: Index, E: Entity> SupernodalLdltRef<'a, I, E> {
    #[inline]
    pub fn new(symbolic: &'a SymbolicSupernodalCholesky<I>, values: SliceGroup<'a, E>) -> Self {
        assert!(values.len() == symbolic.len_values());
        Self { symbolic, values }
    }

    #[inline]
    pub fn symbolic(self) -> &'a SymbolicSupernodalCholesky<I> {
        self.symbolic
    }

    #[inline]
    pub fn values(self) -> SliceGroup<'a, E> {
        self.values
    }
}

impl<'a, I: Index, E: Entity> SimplicialLdltRef<'a, I, E> {
    #[inline]
    pub fn new(symbolic: &'a SymbolicSimplicialCholesky<I>, values: SliceGroup<'a, E>) -> Self {
        assert!(values.len() == symbolic.len_values());
        Self { symbolic, values }
    }

    #[inline]
    pub fn symbolic(self) -> &'a SymbolicSimplicialCholesky<I> {
        self.symbolic
    }

    #[inline]
    pub fn values(self) -> SliceGroup<'a, E> {
        self.values
    }
}

impl<I: Index> SymbolicSupernodalCholesky<I> {
    #[inline]
    pub fn n_supernodes(&self) -> usize {
        self.supernode_postorder.len()
    }

    #[inline]
    pub fn nrows(&self) -> usize {
        self.dimension
    }
    #[inline]
    pub fn ncols(&self) -> usize {
        self.nrows()
    }

    #[inline]
    pub fn len_values(&self) -> usize {
        self.col_ptrs_for_values()[self.n_supernodes()].zx()
    }

    #[inline]
    pub fn supernode_begin(&self) -> &[I] {
        &self.supernode_begin[..self.n_supernodes()]
    }

    #[inline]
    pub fn supernode_end(&self) -> &[I] {
        &self.supernode_begin[1..]
    }

    #[inline]
    pub fn col_ptrs_for_row_indices(&self) -> &[I] {
        &self.col_ptrs_for_row_indices
    }

    #[inline]
    pub fn col_ptrs_for_values(&self) -> &[I] {
        &self.col_ptrs_for_values
    }

    #[inline]
    pub fn row_indices(&self) -> &[I] {
        &self.row_indices
    }
}

impl<I: Index> SymbolicSimplicialCholesky<I> {
    #[inline]
    pub fn nrows(&self) -> usize {
        self.dimension
    }
    #[inline]
    pub fn ncols(&self) -> usize {
        self.nrows()
    }

    #[inline]
    pub fn len_values(&self) -> usize {
        self.row_indices.len()
    }

    #[inline]
    pub fn col_ptrs(&self) -> &[I] {
        &self.col_ptrs
    }

    #[inline]
    pub fn row_indices(&self) -> &[I] {
        &self.row_indices
    }
}

fn postorder_depth_first_search<'n, I: Index>(
    post: &mut Array<'n, I>,
    root: usize,
    mut start_index: usize,
    stack: &mut Array<'n, I>,
    first_child: &mut Array<'n, MaybeIdx<'n, I>>,
    next_child: &Array<'n, I>,
) -> usize {
    let mut top = 1usize;
    let N = post.len();

    stack[N.check(0)] = I::truncate(root);
    while top != 0 {
        let current_node = stack[N.check(top - 1)].zx();
        let first_child = &mut first_child[N.check(current_node)];
        let current_child = first_child.sx();

        if let Some(current_child) = current_child.idx() {
            stack[N.check(top)] = *current_child.truncate::<I>();
            top += 1;
            *first_child = MaybeIdx::new_index_checked(next_child[current_child], N);
        } else {
            post[N.check(start_index)] = I::truncate(current_node);
            start_index += 1;
            top -= 1;
        }
    }
    start_index
}

/// workspace: I×(3*n)
pub fn ghost_postorder<'n, I: Index>(
    post: &mut Array<'n, I>,
    etree: &Array<'n, MaybeIdx<'n, I>>,
    stack: PodStack<'_>,
) {
    let N = post.len();
    let n = *N;

    if n == 0 {
        return;
    }

    let (mut stack_, stack) = stack.make_raw::<I>(n);
    let (mut first_child, stack) = stack.make_raw::<I>(n);
    let (mut next_child, _) = stack.make_raw::<I>(n);

    let stack = Array::from_mut(&mut stack_, N);
    let next_child = Array::from_mut(&mut next_child, N);

    let first_child = Array::from_mut(ghost::fill_none(&mut first_child, N), N);

    for j in N.indices().rev() {
        let parent = etree[j];
        let next = &mut next_child[j];

        if let Some(parent) = parent.idx() {
            let first = &mut first_child[parent.zx()];
            *next = **first;
            *first = MaybeIdx::from_index(j.truncate::<I>());
        }
    }

    let mut start_index = 0usize;
    for (root, &parent) in etree.iter().enumerate() {
        if parent.idx().is_none() {
            start_index = postorder_depth_first_search(
                post,
                root,
                start_index,
                stack,
                first_child,
                next_child,
            );
        }
    }
}

pub fn factorize_supernodal_symbolic_req<I: Index>(n: usize) -> Result<StackReq, SizeOverflow> {
    let n_req = StackReq::try_new::<I>(n)?;
    StackReq::try_all_of([n_req, n_req, n_req, n_req])
}

pub fn ghost_factorize_supernodal_symbolic<'n, I: Index>(
    A: ghost::SymbolicSparseColMatRef<'n, 'n, '_, I>,
    etree: &Array<'n, MaybeIdx<'n, I>>,
    col_counts: &Array<'n, I>,
    stack: PodStack<'_>,
    params: CholeskySymbolicSupernodalParams<'_>,
) -> Result<SymbolicSupernodalCholesky<I>, FaerSparseError> {
    let to_wide = |i: I| i.zx() as u128;
    let from_wide = |i: u128| I::truncate(i as usize);
    let from_wide_checked =
        |i: u128| -> Option<I> { (i <= to_wide(I::MAX)).then_some(I::truncate(i as usize)) };

    let N = A.nrows();
    let n = *N;

    let zero = I::truncate(0);
    let one = I::truncate(1);
    let none = I::truncate(NONE);

    if n == 0 {
        // would be funny if this allocation failed
        return Ok(SymbolicSupernodalCholesky {
            dimension: n,
            supernode_postorder: Vec::new(),
            supernode_postorder_inv: Vec::new(),
            descendent_count: Vec::new(),

            supernode_begin: try_collect([zero])?,
            col_ptrs_for_row_indices: try_collect([zero])?,
            col_ptrs_for_values: try_collect([zero])?,
            row_indices: Vec::new(),
        });
    }
    let mut original_stack = stack;

    let (mut index_to_super__, stack) = original_stack.rb_mut().make_raw::<I>(n);
    let (mut super_etree__, stack) = stack.make_raw::<I>(n);
    let (mut supernode_sizes__, stack) = stack.make_raw::<I>(n);
    let (mut child_count__, _) = stack.make_raw::<I>(n);

    let child_count = Array::from_mut(&mut child_count__, N);
    let index_to_super = Array::from_mut(&mut index_to_super__, N);

    mem::fill_zero(child_count);
    for j in N.indices() {
        if let Some(parent) = etree[j].idx() {
            child_count[parent.zx()].incr();
        }
    }

    mem::fill_zero(&mut supernode_sizes__);
    let mut current_supernode = 0usize;
    supernode_sizes__[0] = one;
    for (j_prev, j) in zip(N.indices().take(n - 1), N.indices().skip(1)) {
        let is_parent_of_prev = (*etree[j_prev]).sx() == *j;
        let is_parent_of_only_prev = child_count[j] == one;
        let same_pattern_as_prev = col_counts[j_prev] == col_counts[j] + one;

        if !(is_parent_of_prev && is_parent_of_only_prev && same_pattern_as_prev) {
            current_supernode += 1;
        }
        supernode_sizes__[current_supernode].incr();
    }
    let n_fundamental_supernodes = current_supernode + 1;

    // last n elements contain supernode degrees
    let supernode_begin__ = ghost::with_size(
        n_fundamental_supernodes,
        |N_FUNDAMENTAL_SUPERNODES| -> Result<Vec<I>, FaerSparseError> {
            let supernode_sizes = Array::from_mut(
                &mut supernode_sizes__[..n_fundamental_supernodes],
                N_FUNDAMENTAL_SUPERNODES,
            );
            let super_etree = Array::from_mut(
                &mut super_etree__[..n_fundamental_supernodes],
                N_FUNDAMENTAL_SUPERNODES,
            );

            let mut supernode_begin = 0usize;
            for s in N_FUNDAMENTAL_SUPERNODES.indices() {
                let size = supernode_sizes[s].zx();
                (**index_to_super)[supernode_begin..][..size].fill(*s.truncate::<I>());
                supernode_begin += size;
            }

            let index_to_super = Array::from_mut(
                Idx::slice_mut_checked(index_to_super, N_FUNDAMENTAL_SUPERNODES),
                N,
            );

            let mut supernode_begin = 0usize;
            for s in N_FUNDAMENTAL_SUPERNODES.indices() {
                let size = supernode_sizes[s].zx();
                let last = supernode_begin + size - 1;
                let last = N.check(last);
                if let Some(parent) = etree[last].idx() {
                    super_etree[s] = *index_to_super[parent.zx()];
                } else {
                    super_etree[s] = none;
                }
                supernode_begin += size;
            }

            let super_etree = Array::from_mut(
                MaybeIdx::slice_mut_checked(super_etree, N_FUNDAMENTAL_SUPERNODES),
                N_FUNDAMENTAL_SUPERNODES,
            );

            if let Some(relax) = params.relax {
                let req = || -> Result<StackReq, SizeOverflow> {
                    let req = StackReq::try_new::<I>(n_fundamental_supernodes)?;
                    StackReq::try_all_of([req; 5])
                };
                let mut mem =
                    dyn_stack::GlobalPodBuffer::try_new(req().map_err(nomem)?).map_err(nomem)?;
                let stack = PodStack::new(&mut mem);

                let child_lists = &mut (**child_count)[..n_fundamental_supernodes];
                let (mut child_list_heads, stack) = stack.make_raw::<I>(n_fundamental_supernodes);
                let (mut last_merged_children, stack) =
                    stack.make_raw::<I>(n_fundamental_supernodes);
                let (mut merge_parents, stack) = stack.make_raw::<I>(n_fundamental_supernodes);
                let (mut fundamental_supernode_degrees, stack) =
                    stack.make_raw::<I>(n_fundamental_supernodes);
                let (mut num_zeros, _) = stack.make_raw::<I>(n_fundamental_supernodes);

                let child_lists = Array::from_mut(
                    ghost::fill_none(child_lists, N_FUNDAMENTAL_SUPERNODES),
                    N_FUNDAMENTAL_SUPERNODES,
                );
                let child_list_heads = Array::from_mut(
                    ghost::fill_none(&mut child_list_heads, N_FUNDAMENTAL_SUPERNODES),
                    N_FUNDAMENTAL_SUPERNODES,
                );
                let last_merged_children = Array::from_mut(
                    ghost::fill_none(&mut last_merged_children, N_FUNDAMENTAL_SUPERNODES),
                    N_FUNDAMENTAL_SUPERNODES,
                );
                let merge_parents = Array::from_mut(
                    ghost::fill_none(&mut merge_parents, N_FUNDAMENTAL_SUPERNODES),
                    N_FUNDAMENTAL_SUPERNODES,
                );
                let fundamental_supernode_degrees =
                    Array::from_mut(&mut fundamental_supernode_degrees, N_FUNDAMENTAL_SUPERNODES);
                let num_zeros = Array::from_mut(&mut num_zeros, N_FUNDAMENTAL_SUPERNODES);

                let mut supernode_begin = 0usize;
                for s in N_FUNDAMENTAL_SUPERNODES.indices() {
                    let size = supernode_sizes[s].zx();
                    fundamental_supernode_degrees[s] =
                        col_counts[N.check(supernode_begin + size - 1)] - one;
                    supernode_begin += size;
                }

                for s in N_FUNDAMENTAL_SUPERNODES.indices() {
                    if let Some(parent) = super_etree[s].idx() {
                        let parent = parent.zx();
                        child_lists[s] = child_list_heads[parent];
                        child_list_heads[parent] = MaybeIdx::from_index(s.truncate());
                    }
                }

                mem::fill_zero(num_zeros);
                for parent in N_FUNDAMENTAL_SUPERNODES.indices() {
                    loop {
                        let mut merging_child = MaybeIdx::none();
                        let mut num_new_zeros = 0usize;
                        let mut num_merged_zeros = 0usize;
                        let mut largest_mergable_size = 0usize;

                        let mut child_ = child_list_heads[parent];
                        while let Some(child) = child_.idx() {
                            let child = child.zx();
                            if *child + 1 != *parent {
                                child_ = child_lists[child];
                                continue;
                            }

                            if merge_parents[child].idx().is_some() {
                                child_ = child_lists[child];
                                continue;
                            }

                            let parent_size = supernode_sizes[parent].zx();
                            let child_size = supernode_sizes[child].zx();
                            if child_size < largest_mergable_size {
                                child_ = child_lists[child];
                                continue;
                            }

                            let parent_degree = fundamental_supernode_degrees[parent].zx();
                            let child_degree = fundamental_supernode_degrees[child].zx();

                            let num_parent_zeros = num_zeros[parent].zx();
                            let num_child_zeros = num_zeros[child].zx();

                            let status_num_merged_zeros = {
                                let num_new_zeros =
                                    (parent_size + parent_degree - child_degree) * child_size;

                                if num_new_zeros == 0 {
                                    num_parent_zeros + num_child_zeros
                                } else {
                                    let num_old_zeros = num_child_zeros + num_parent_zeros;
                                    let num_zeros = num_new_zeros + num_old_zeros;

                                    let combined_size = child_size + parent_size;
                                    let num_expanded_entries =
                                        (combined_size * (combined_size + 1)) / 2
                                            + parent_degree * combined_size;

                                    let f = || {
                                        for cutoff in relax {
                                            let num_zeros_cutoff =
                                                num_expanded_entries as f64 * cutoff.1;
                                            if cutoff.0 >= combined_size
                                                && num_zeros_cutoff >= num_zeros as f64
                                            {
                                                return num_zeros;
                                            }
                                        }
                                        NONE
                                    };
                                    f()
                                }
                            };
                            if status_num_merged_zeros == NONE {
                                child_ = child_lists[child];
                                continue;
                            }

                            let num_proposed_new_zeros =
                                status_num_merged_zeros - (num_child_zeros + num_parent_zeros);
                            if child_size > largest_mergable_size
                                || num_proposed_new_zeros < num_new_zeros
                            {
                                merging_child = MaybeIdx::from_index(child);
                                num_new_zeros = num_proposed_new_zeros;
                                num_merged_zeros = status_num_merged_zeros;
                                largest_mergable_size = child_size;
                            }

                            child_ = child_lists[child];
                        }

                        if let Some(merging_child) = merging_child.idx() {
                            supernode_sizes[parent] =
                                supernode_sizes[parent] + supernode_sizes[merging_child];
                            supernode_sizes[merging_child] = zero;
                            num_zeros[parent] = I::truncate(num_merged_zeros);

                            merge_parents[merging_child] =
                                if let Some(child) = last_merged_children[parent].idx() {
                                    MaybeIdx::from_index(child)
                                } else {
                                    MaybeIdx::from_index(parent.truncate())
                                };

                            last_merged_children[parent] =
                                if let Some(child) = last_merged_children[merging_child].idx() {
                                    MaybeIdx::from_index(child)
                                } else {
                                    MaybeIdx::from_index(merging_child.truncate())
                                };
                        } else {
                            break;
                        }
                    }
                }

                let original_to_relaxed = last_merged_children;
                original_to_relaxed.fill(MaybeIdx::none_index());

                let mut pos = 0usize;
                for s in N_FUNDAMENTAL_SUPERNODES.indices() {
                    let idx = N_FUNDAMENTAL_SUPERNODES.check(pos);
                    let size = supernode_sizes[s];
                    let degree = fundamental_supernode_degrees[s];
                    if size > zero {
                        supernode_sizes[idx] = size;
                        fundamental_supernode_degrees[idx] = degree;
                        original_to_relaxed[s] = MaybeIdx::from_index(idx.truncate());

                        pos += 1;
                    }
                }
                let n_relaxed_supernodes = pos;

                let mut supernode_begin__ = try_zeroed(n_relaxed_supernodes + 1)?;
                supernode_begin__[1..]
                    .copy_from_slice(&(**fundamental_supernode_degrees)[..n_relaxed_supernodes]);

                Ok(supernode_begin__)
            } else {
                let mut supernode_begin__ = try_zeroed(n_fundamental_supernodes + 1)?;

                let mut supernode_begin = 0usize;
                for s in N_FUNDAMENTAL_SUPERNODES.indices() {
                    let size = supernode_sizes[s].zx();
                    supernode_begin__[*s + 1] =
                        col_counts[N.check(supernode_begin + size - 1)] - one;
                    supernode_begin += size;
                }

                Ok(supernode_begin__)
            }
        },
    )?;

    let n_supernodes = supernode_begin__.len() - 1;

    let (supernode_begin__, col_ptrs_for_row_indices__, col_ptrs_for_values__, row_indices__) =
        ghost::with_size(
            n_supernodes,
            |N_SUPERNODES| -> Result<(Vec<I>, Vec<I>, Vec<I>, Vec<I>), FaerSparseError> {
                let supernode_sizes =
                    Array::from_mut(&mut supernode_sizes__[..n_supernodes], N_SUPERNODES);

                if n_supernodes != n_fundamental_supernodes {
                    let mut supernode_begin = 0usize;
                    for s in N_SUPERNODES.indices() {
                        let size = supernode_sizes[s].zx();
                        (**index_to_super)[supernode_begin..][..size].fill(*s.truncate::<I>());
                        supernode_begin += size;
                    }

                    let index_to_super =
                        Array::from_mut(Idx::slice_mut_checked(index_to_super, N_SUPERNODES), N);
                    let super_etree =
                        Array::from_mut(&mut super_etree__[..n_supernodes], N_SUPERNODES);

                    let mut supernode_begin = 0usize;
                    for s in N_SUPERNODES.indices() {
                        let size = supernode_sizes[s].zx();
                        let last = supernode_begin + size - 1;
                        if let Some(parent) = etree[N.check(last)].idx() {
                            super_etree[s] = *index_to_super[parent.zx()];
                        } else {
                            super_etree[s] = none;
                        }
                        supernode_begin += size;
                    }
                }

                let index_to_super =
                    Array::from_mut(Idx::slice_mut_checked(index_to_super, N_SUPERNODES), N);

                let mut supernode_begin__ = supernode_begin__;
                let mut col_ptrs_for_row_indices__ = try_zeroed::<I>(n_supernodes + 1)?;
                let mut col_ptrs_for_values__ = try_zeroed::<I>(n_supernodes + 1)?;

                let mut row_ptr = zero;
                let mut val_ptr = zero;

                supernode_begin__[0] = zero;

                let mut row_indices__ = {
                    let mut wide_val_count = 0u128;
                    for (s, [current, next]) in zip(
                        N_SUPERNODES.indices(),
                        windows2(Cell::as_slice_of_cells(Cell::from_mut(
                            &mut *supernode_begin__,
                        ))),
                    ) {
                        let degree = next.get();
                        let ncols = supernode_sizes[s];
                        let nrows = degree + ncols;
                        supernode_sizes[s] = row_ptr;
                        next.set(current.get() + ncols);

                        col_ptrs_for_row_indices__[*s] = row_ptr;
                        col_ptrs_for_values__[*s] = val_ptr;

                        let wide_matrix_size = to_wide(nrows) * to_wide(ncols);
                        wide_val_count += wide_matrix_size;

                        row_ptr += degree;
                        val_ptr = from_wide(to_wide(val_ptr) + wide_matrix_size);
                    }
                    col_ptrs_for_row_indices__[n_supernodes] = row_ptr;
                    col_ptrs_for_values__[n_supernodes] = val_ptr;
                    from_wide_checked(wide_val_count).ok_or(FaerSparseError::IndexOverflow)?;

                    try_zeroed::<I>(row_ptr.zx())?
                };

                let super_etree = Array::from_ref(
                    MaybeIdx::slice_ref_checked(&super_etree__[..n_supernodes], N_SUPERNODES),
                    N_SUPERNODES,
                );

                let current_row_positions = supernode_sizes;

                let row_indices = Idx::slice_mut_checked(&mut row_indices__, N);
                let visited = Array::from_mut(&mut (**child_count)[..n_supernodes], N_SUPERNODES);
                mem::fill_none(visited);
                for s in N_SUPERNODES.indices() {
                    let k1 = ghost::IdxInclusive::new_checked(supernode_begin__[*s].zx(), N);
                    let k2 = ghost::IdxInclusive::new_checked(supernode_begin__[*s + 1].zx(), N);

                    for k in k1.range_to(k2) {
                        ereach_super(
                            A,
                            super_etree,
                            index_to_super,
                            current_row_positions,
                            row_indices,
                            k,
                            visited,
                        );
                    }
                }

                debug_assert!(**current_row_positions == col_ptrs_for_row_indices__[1..]);

                Ok((
                    supernode_begin__,
                    col_ptrs_for_row_indices__,
                    col_ptrs_for_values__,
                    row_indices__,
                ))
            },
        )?;

    let mut supernode_etree__ = try_collect(super_etree__[..n_supernodes].iter().copied())?;
    let mut supernode_postorder__ = try_zeroed::<I>(n_supernodes)?;

    drop(super_etree__);
    drop(child_count__);
    drop(supernode_sizes__);
    drop(index_to_super__);

    let mut descendent_count__ = try_zeroed::<I>(n_supernodes)?;

    ghost::with_size(n_supernodes, |N_SUPERNODES| {
        let post = Array::from_mut(&mut supernode_postorder__, N_SUPERNODES);
        let desc_count = Array::from_mut(&mut descendent_count__, N_SUPERNODES);
        let etree = Array::from_ref(
            MaybeIdx::slice_ref_checked(&supernode_etree__, N_SUPERNODES),
            N_SUPERNODES,
        );

        for s in N_SUPERNODES.indices() {
            if let Some(parent) = etree[s].idx() {
                let parent = parent.zx();
                desc_count[parent] = desc_count[parent] + desc_count[s] + one;
            }
        }

        ghost_postorder(post, etree, original_stack);
        let post_inv = Array::from_mut(&mut supernode_etree__, N_SUPERNODES);
        for i in N_SUPERNODES.indices() {
            post_inv[N_SUPERNODES.check(post[i].zx())] = *i.truncate();
        }
    });

    Ok(SymbolicSupernodalCholesky {
        dimension: n,
        supernode_postorder: supernode_postorder__,
        supernode_postorder_inv: supernode_etree__,
        descendent_count: descendent_count__,
        supernode_begin: supernode_begin__,
        col_ptrs_for_row_indices: col_ptrs_for_row_indices__,
        col_ptrs_for_values: col_ptrs_for_values__,
        row_indices: row_indices__,
    })
}

#[inline]
fn partition_fn<I: Index>(idx: usize) -> impl Fn(&I) -> bool {
    let idx = I::truncate(idx);
    move |&i| i < idx
}

pub fn factorize_simplicial_numeric_ldlt_req<I: Index, E: Entity>(
    n: usize,
) -> Result<StackReq, SizeOverflow> {
    let n_req = StackReq::try_new::<I>(n)?;
    StackReq::try_all_of([make_raw_req::<E>(n)?, n_req, n_req, n_req])
}

pub fn factorize_supernodal_numeric_ldlt_req<I: Index, E: Entity>(
    symbolic: &SymbolicSupernodalCholesky<I>,
    parallelism: Parallelism,
) -> Result<StackReq, SizeOverflow> {
    let n_supernodes = symbolic.n_supernodes();
    let n = symbolic.nrows();
    let post = &*symbolic.supernode_postorder;
    let post_inv = &*symbolic.supernode_postorder_inv;

    let desc_count = &*symbolic.descendent_count;

    let col_ptr_row = &*symbolic.col_ptrs_for_row_indices;
    let row_ind = &*symbolic.row_indices;

    let mut req = StackReq::empty();
    for s in 0..n_supernodes {
        let s_start = symbolic.supernode_begin[s].zx();
        let s_end = symbolic.supernode_begin[s + 1].zx();

        let s_ncols = s_end - s_start;

        let s_postordered = post_inv[s].zx();
        let desc_count = desc_count[s].zx();
        for d in &post[s_postordered - desc_count..s_postordered] {
            let mut d_req = StackReq::empty();

            let d = d.zx();
            let d_start = symbolic.supernode_begin[d].zx();
            let d_end = symbolic.supernode_begin[d + 1].zx();

            let d_pattern = &row_ind[col_ptr_row[d].zx()..col_ptr_row[d + 1].zx()];

            let d_ncols = d_end - d_start;

            let d_pattern_start = d_pattern.partition_point(partition_fn(s_start));
            let d_pattern_mid_len =
                d_pattern[d_pattern_start..].partition_point(partition_fn(s_end));

            d_req = d_req.try_and(temp_mat_req::<E>(
                d_pattern.len() - d_pattern_start,
                d_pattern_mid_len,
            )?)?;
            d_req = d_req.try_and(temp_mat_req::<E>(d_ncols, d_pattern_mid_len)?)?;
            req = req.try_or(d_req)?;
        }
        req = req.try_or(
            faer_cholesky::ldlt_diagonal::compute::raw_cholesky_in_place_req::<E>(
                s_ncols,
                parallelism,
                Default::default(),
            )?,
        )?;
    }
    req.try_and(StackReq::try_new::<I>(n)?)
}

pub fn factorize_supernodal_numeric_ldlt<I: Index, E: ComplexField>(
    L_values: SliceGroupMut<'_, E>,
    A_lower: SparseColMatRef<'_, I, E>,
    symbolic: &SymbolicSupernodalCholesky<I>,
    parallelism: Parallelism,
    stack: PodStack<'_>,
) {
    let n_supernodes = symbolic.n_supernodes();
    let n = symbolic.nrows();
    let mut L_values = L_values;

    assert!(A_lower.nrows() == n);
    assert!(A_lower.ncols() == n);
    assert!(L_values.len() == symbolic.len_values());

    let none = I::truncate(NONE);

    let post = &*symbolic.supernode_postorder;
    let post_inv = &*symbolic.supernode_postorder_inv;

    let desc_count = &*symbolic.descendent_count;

    let col_ptr_row = &*symbolic.col_ptrs_for_row_indices;
    let col_ptr_val = &*symbolic.col_ptrs_for_values;
    let row_ind = &*symbolic.row_indices;

    // mapping from global indices to local
    let (mut global_to_local, mut stack) = stack.make_raw::<I>(n);
    mem::fill_none(&mut global_to_local);

    for s in 0..n_supernodes {
        let s_start = symbolic.supernode_begin[s].zx();
        let s_end = symbolic.supernode_begin[s + 1].zx();

        let s_pattern = &row_ind[col_ptr_row[s].zx()..col_ptr_row[s + 1].zx()];
        let s_ncols = s_end - s_start;
        let s_nrows = s_pattern.len() + s_ncols;

        for (i, &row) in s_pattern.iter().enumerate() {
            global_to_local[row.zx()] = I::truncate(i + s_ncols);
        }

        let (head, tail) = L_values.rb_mut().split_at(col_ptr_val[s].zx());
        let head = head.rb();
        let mut Ls = MatMut::<E>::from_column_major_slice(
            tail.subslice(0..(col_ptr_val[s + 1] - col_ptr_val[s]).zx())
                .into_inner(),
            s_nrows,
            s_ncols,
        );

        for j in s_start..s_end {
            let j_shifted = j - s_start;
            for (i, val) in zip(
                A_lower.row_indices_of_col(j),
                A_lower.values_of_col(j).into_iter(),
            ) {
                let val = val.read();
                if i >= s_end {
                    Ls.write(global_to_local[i].sx(), j_shifted, val);
                } else if i >= j {
                    Ls.write(i - s_start, j_shifted, val);
                }
            }
        }

        let s_postordered = post_inv[s].zx();
        let desc_count = desc_count[s].zx();
        for d in &post[s_postordered - desc_count..s_postordered] {
            let d = d.zx();
            let d_start = symbolic.supernode_begin[d].zx();
            let d_end = symbolic.supernode_begin[d + 1].zx();

            let d_pattern = &row_ind[col_ptr_row[d].zx()..col_ptr_row[d + 1].zx()];
            let d_ncols = d_end - d_start;
            let d_nrows = d_pattern.len() + d_ncols;

            let Ld = MatRef::<E>::from_column_major_slice(
                head.subslice(col_ptr_val[d].zx()..col_ptr_val[d + 1].zx())
                    .into_inner(),
                d_nrows,
                d_ncols,
            );

            let d_pattern_start = d_pattern.partition_point(partition_fn(s_start));
            let d_pattern_mid_len =
                d_pattern[d_pattern_start..].partition_point(partition_fn(s_end));
            let d_pattern_mid = d_pattern_start + d_pattern_mid_len;

            let [Ld_top, Ld_mid_bot] = Ld.split_at_row(d_ncols);
            let [_, Ld_mid_bot] = Ld_mid_bot.split_at_row(d_pattern_start);
            let [Ld_mid, Ld_bot] = Ld_mid_bot.split_at_row(d_pattern_mid_len);
            let D = Ld_top.diagonal();

            let stack = stack.rb_mut();

            let (mut tmp, stack) =
                temp_mat_uninit::<E>(Ld_mid_bot.nrows(), d_pattern_mid_len, stack);
            let tmp = tmp.as_mut();
            let (mut tmp2, _) = temp_mat_uninit::<E>(Ld_mid.ncols(), Ld_mid.nrows(), stack);
            let mut Ld_mid_x_D = tmp2.as_mut().transpose();

            for i in 0..d_pattern_mid_len {
                for j in 0..d_ncols {
                    Ld_mid_x_D.write(i, j, Ld_mid.read(i, j).scale_real(D.read(j, 0).real()));
                }
            }

            let [mut tmp_top, mut tmp_bot] = tmp.split_at_row(d_pattern_mid_len);

            use faer_core::{mul, mul::triangular};
            triangular::matmul(
                tmp_top.rb_mut(),
                triangular::BlockStructure::TriangularLower,
                Ld_mid,
                triangular::BlockStructure::Rectangular,
                Ld_mid_x_D.rb().adjoint(),
                triangular::BlockStructure::Rectangular,
                None,
                E::one(),
                parallelism,
            );
            mul::matmul(
                tmp_bot.rb_mut(),
                Ld_bot,
                Ld_mid_x_D.rb().adjoint(),
                None,
                E::one(),
                parallelism,
            );
            for (j_idx, j) in d_pattern[d_pattern_start..d_pattern_mid].iter().enumerate() {
                let j = j.zx();
                let j_s = j - s_start;
                for (i_idx, i) in d_pattern[d_pattern_start..d_pattern_mid][j_idx..]
                    .iter()
                    .enumerate()
                {
                    let i_idx = i_idx + j_idx;

                    let i = i.zx();
                    let i_s = i - s_start;

                    debug_assert!(i_s >= j_s);

                    Ls.write(i_s, j_s, Ls.read(i_s, j_s).sub(tmp_top.read(i_idx, j_idx)));
                }
            }

            for (j_idx, j) in d_pattern[d_pattern_start..d_pattern_mid].iter().enumerate() {
                let j = j.zx();
                let j_s = j - s_start;
                for (i_idx, i) in d_pattern[d_pattern_mid..].iter().enumerate() {
                    let i = i.zx();
                    let i_s = global_to_local[i].zx();
                    Ls.write(i_s, j_s, Ls.read(i_s, j_s).sub(tmp_bot.read(i_idx, j_idx)));
                }
            }
        }

        let [mut Ls_top, mut Ls_bot] = Ls.rb_mut().split_at_row(s_ncols);

        let params = Default::default();
        faer_cholesky::ldlt_diagonal::compute::raw_cholesky_in_place(
            Ls_top.rb_mut(),
            parallelism,
            stack.rb_mut(),
            params,
        );
        zipped!(Ls_top.rb_mut())
            .for_each_triangular_upper(faer_core::zip::Diag::Skip, |mut x| x.write(E::zero()));
        faer_core::solve::solve_unit_lower_triangular_in_place(
            Ls_top.rb().conjugate(),
            Ls_bot.rb_mut().transpose(),
            parallelism,
        );
        for j in 0..s_ncols {
            let d = Ls_top.read(j, j).real().inv();
            for i in 0..s_pattern.len() {
                Ls_bot.write(i, j, Ls_bot.read(i, j).scale_real(d));
            }
        }

        for &row in s_pattern {
            global_to_local[row.zx()] = none;
        }
    }
}

pub fn ghost_transpose_symbolic<'m, 'n, 'a, I: Index>(
    new_col_ptrs: &'a mut [I],
    new_row_indices: &'a mut [I],
    A: ghost::SymbolicSparseColMatRef<'m, 'n, '_, I>,
    stack: PodStack<'_>,
) -> ghost::SymbolicSparseColMatRef<'n, 'm, 'a, I> {
    let M = A.nrows();
    let N = A.ncols();
    assert!(new_col_ptrs.len() == *M + 1);

    let (mut col_count, _) = stack.make_raw::<I>(*M);
    let col_count = Array::from_mut(&mut col_count, M);
    mem::fill_zero(col_count);

    // can't overflow because the total count is A.compute_nnz() <= I::MAX
    let col_count = &mut *col_count;
    if A.nnz_per_col().is_some() {
        for j in N.indices() {
            for i in A.row_indices_of_col(j) {
                col_count[i].incr();
            }
        }
    } else {
        for i in A.compressed_row_indices() {
            col_count[i].incr();
        }
    }

    // col_count elements are >= 0
    for (j, [pj0, pj1]) in zip(
        M.indices(),
        windows2(Cell::as_slice_of_cells(Cell::from_mut(new_col_ptrs))),
    ) {
        let cj = &mut col_count[j];
        let pj = pj0.get();
        // new_col_ptrs is non-decreasing
        pj1.set(pj + *cj);
        *cj = pj;
    }

    let new_row_indices = &mut new_row_indices[..new_col_ptrs[*M].zx()];
    let current_row_position = &mut *col_count;
    // current_row_position[i] == col_ptr[i]
    for j in N.indices() {
        let j_: Idx<'n, I> = j.truncate::<I>();
        for i in A.row_indices_of_col(j) {
            let ci = &mut current_row_position[i];

            // SAFETY: see below
            *unsafe { new_row_indices.get_unchecked_mut(ci.zx()) } = *j_;
            ci.incr();
        }
    }
    // current_row_position[i] == col_ptr[i] + col_count[i] == col_ptr[i + 1] <= col_ptr[m]
    // so all the unchecked accesses were valid and non-overlapping, which means the entire
    // array is filled
    debug_assert!(&**current_row_position == &new_col_ptrs[1..]);

    // SAFETY:
    // 0. new_col_ptrs is non-decreasing (see ghost_permute_symmetric_common)
    // 1. all written row indices are less than n
    ghost::SymbolicSparseColMatRef::new(
        unsafe {
            SymbolicSparseColMatRef::new_unchecked(*N, *M, new_col_ptrs, None, new_row_indices)
        },
        N,
        M,
    )
}

pub fn ghost_adjoint<'m, 'n, 'a, I: Index, E: ComplexField>(
    new_col_ptrs: &'a mut [I],
    new_row_indices: &'a mut [I],
    new_values: SliceGroupMut<'a, E>,
    A: ghost::SparseColMatRef<'m, 'n, '_, I, E>,
    stack: PodStack<'_>,
) -> ghost::SparseColMatRef<'n, 'm, 'a, I, E> {
    let M = A.nrows();
    let N = A.ncols();
    assert!(new_col_ptrs.len() == *M + 1);

    let (mut col_count, _) = stack.make_raw::<I>(*M);
    let col_count = Array::from_mut(&mut col_count, M);
    mem::fill_zero(col_count);

    // can't overflow because the total count is A.compute_nnz() <= I::MAX
    let col_count = &mut *col_count;
    if A.nnz_per_col().is_some() {
        for j in N.indices() {
            for i in A.row_indices_of_col(j) {
                col_count[i].incr();
            }
        }
    } else {
        for i in A.symbolic().compressed_row_indices() {
            col_count[i].incr();
        }
    }

    // col_count elements are >= 0
    for (j, [pj0, pj1]) in zip(
        M.indices(),
        windows2(Cell::as_slice_of_cells(Cell::from_mut(new_col_ptrs))),
    ) {
        let cj = &mut col_count[j];
        let pj = pj0.get();
        // new_col_ptrs is non-decreasing
        pj1.set(pj + *cj);
        *cj = pj;
    }

    let new_row_indices = &mut new_row_indices[..new_col_ptrs[*M].zx()];
    let mut new_values = new_values.subslice(0..new_col_ptrs[*M].zx());
    let current_row_position = &mut *col_count;
    // current_row_position[i] == col_ptr[i]
    for j in N.indices() {
        let j_: Idx<'n, I> = j.truncate::<I>();
        for (i, val) in zip(A.row_indices_of_col(j), A.values_of_col(j).into_iter()) {
            let ci = &mut current_row_position[i];

            // SAFETY: see below
            unsafe {
                *new_row_indices.get_unchecked_mut(ci.zx()) = *j_;
                new_values.write_unchecked(ci.zx(), val.read().conj())
            };
            ci.incr();
        }
    }
    // current_row_position[i] == col_ptr[i] + col_count[i] == col_ptr[i + 1] <= col_ptr[m]
    // so all the unchecked accesses were valid and non-overlapping, which means the entire
    // array is filled
    debug_assert!(&**current_row_position == &new_col_ptrs[1..]);

    // SAFETY:
    // 0. new_col_ptrs is non-decreasing (see ghost_permute_symmetric_common)
    // 1. all written row indices are less than n
    ghost::SparseColMatRef::new(
        unsafe {
            SparseColMatRef::new(
                SymbolicSparseColMatRef::new_unchecked(*N, *M, new_col_ptrs, None, new_row_indices),
                new_values.into_const(),
            )
        },
        N,
        M,
    )
}

pub fn ghost_transpose<'m, 'n, 'a, I: Index, E: Entity>(
    new_col_ptrs: &'a mut [I],
    new_row_indices: &'a mut [I],
    new_values: SliceGroupMut<'a, E>,
    A: ghost::SparseColMatRef<'m, 'n, '_, I, E>,
    stack: PodStack<'_>,
) -> ghost::SparseColMatRef<'n, 'm, 'a, I, E> {
    let M = A.nrows();
    let N = A.ncols();
    assert!(new_col_ptrs.len() == *M + 1);

    let (mut col_count, _) = stack.make_raw::<I>(*M);
    let col_count = Array::from_mut(&mut col_count, M);
    mem::fill_zero(col_count);

    // can't overflow because the total count is A.compute_nnz() <= I::MAX
    let col_count = &mut *col_count;
    if A.nnz_per_col().is_some() {
        for j in N.indices() {
            for i in A.row_indices_of_col(j) {
                col_count[i].incr();
            }
        }
    } else {
        for i in A.symbolic().compressed_row_indices() {
            col_count[i].incr();
        }
    }

    // col_count elements are >= 0
    for (j, [pj0, pj1]) in zip(
        M.indices(),
        windows2(Cell::as_slice_of_cells(Cell::from_mut(new_col_ptrs))),
    ) {
        let cj = &mut col_count[j];
        let pj = pj0.get();
        // new_col_ptrs is non-decreasing
        pj1.set(pj + *cj);
        *cj = pj;
    }

    let new_row_indices = &mut new_row_indices[..new_col_ptrs[*M].zx()];
    let mut new_values = new_values.subslice(0..new_col_ptrs[*M].zx());
    let current_row_position = &mut *col_count;
    // current_row_position[i] == col_ptr[i]
    for j in N.indices() {
        let j_: Idx<'n, I> = j.truncate::<I>();
        for (i, val) in zip(A.row_indices_of_col(j), A.values_of_col(j).into_iter()) {
            let ci = &mut current_row_position[i];

            // SAFETY: see below
            unsafe {
                *new_row_indices.get_unchecked_mut(ci.zx()) = *j_;
                new_values.write_unchecked(ci.zx(), val.read())
            };
            ci.incr();
        }
    }
    // current_row_position[i] == col_ptr[i] + col_count[i] == col_ptr[i + 1] <= col_ptr[m]
    // so all the unchecked accesses were valid and non-overlapping, which means the entire
    // array is filled
    debug_assert!(&**current_row_position == &new_col_ptrs[1..]);

    // SAFETY:
    // 0. new_col_ptrs is non-decreasing (see ghost_permute_symmetric_common)
    // 1. all written row indices are less than n
    ghost::SparseColMatRef::new(
        unsafe {
            SparseColMatRef::new(
                SymbolicSparseColMatRef::new_unchecked(*N, *M, new_col_ptrs, None, new_row_indices),
                new_values.into_const(),
            )
        },
        N,
        M,
    )
}

#[derive(Copy, Clone, Debug)]
pub struct CholeskySymbolicParams<'a> {
    pub amd_params: Control,
    pub supernodal_flop_ratio_threshold: f64,
    pub supernodal_params: CholeskySymbolicSupernodalParams<'a>,
}

impl Default for CholeskySymbolicParams<'_> {
    fn default() -> Self {
        Self {
            supernodal_flop_ratio_threshold: 40.0,
            amd_params: Default::default(),
            supernodal_params: Default::default(),
        }
    }
}

pub fn factorize_symbolic<I: Index>(
    A: SymbolicSparseColMatRef<'_, I>,
    side: Side,
    params: CholeskySymbolicParams<'_>,
) -> Result<SymbolicCholesky<I>, FaerSparseError> {
    let n = A.nrows();
    let A_nnz = A.compute_nnz();

    assert!(A.nrows() == A.ncols());
    let lower = (side == Side::Lower) as usize;

    ghost::with_size(n, |N| {
        let A = ghost::SymbolicSparseColMatRef::new(A, N, N);

        let req = || -> Result<StackReq, SizeOverflow> {
            let n_req = StackReq::try_new::<I>(n)?;
            let A_req = StackReq::try_and(
                // new_col_ptr
                StackReq::try_new::<I>(n + 1)?,
                // new_row_ind
                StackReq::try_new::<I>(A_nnz)?,
            )?;
            let A_req2 = if side == Side::Lower {
                A_req
            } else {
                StackReq::empty()
            };

            StackReq::try_or(
                amd::order_maybe_unsorted_req::<I>(n, A_nnz)?,
                StackReq::try_all_of([
                    A_req,
                    A_req2,
                    // permute_symmetric | etree
                    n_req,
                    // col_counts
                    n_req,
                    // ghost_prefactorize_symbolic
                    n_req,
                    // ghost_factorize_*_symbolic
                    StackReq::try_or(
                        factorize_supernodal_symbolic_req::<I>(n)?,
                        factorize_simplicial_symbolic_req::<I>(n)?,
                    )?,
                ])?,
            )
        };

        let req = req().map_err(nomem)?;
        let mut mem = dyn_stack::GlobalPodBuffer::try_new(req).map_err(nomem)?;
        let mut stack = PodStack::new(&mut mem);

        let mut perm_fwd = try_zeroed(n)?;
        let mut perm_inv = try_zeroed(n)?;
        let flops = amd::order_maybe_unsorted(
            &mut perm_fwd,
            &mut perm_inv,
            *A,
            params.amd_params,
            stack.rb_mut(),
        )?;
        let flops = flops.n_div + flops.n_mult_subs_ldl;
        let perm_ =
            ghost::PermutationRef::new(PermutationRef::new_checked(&perm_fwd, &perm_inv), N);

        let (mut new_col_ptr, stack) = stack.make_raw::<I>(lower * (n + 1));
        let (mut new_row_ind, mut stack) = stack.make_raw::<I>(lower * (A_nnz));

        let A = if side == Side::Lower {
            ghost_transpose_symbolic(&mut new_col_ptr, &mut new_row_ind, A, stack.rb_mut())
        } else {
            A
        };

        let (mut new_col_ptr, stack) = stack.make_raw::<I>(n + 1);
        let (mut new_row_ind, mut stack) = stack.make_raw::<I>(A_nnz);
        let A = ghost_permute_symmetric_symbolic(
            &mut new_col_ptr,
            &mut new_row_ind,
            A,
            perm_,
            stack.rb_mut(),
        );

        let (mut etree, stack) = stack.make_raw::<I>(n);
        let (mut col_counts, mut stack) = stack.make_raw::<I>(n);
        let etree = Array::from_mut(&mut etree, N);
        let col_counts = Array::from_mut(&mut col_counts, N);
        let etree = &*ghost_prefactorize_symbolic(etree, col_counts, A, stack.rb_mut());
        let L_nnz = I::sum_nonnegative(col_counts).ok_or(FaerSparseError::IndexOverflow)?;

        let raw = if (flops / L_nnz.zx() as f64) > params.supernodal_flop_ratio_threshold {
            SymbolicCholeskyRaw::Supernodal(ghost_factorize_supernodal_symbolic(
                A,
                etree,
                col_counts,
                stack.rb_mut(),
                params.supernodal_params,
            )?)
        } else {
            SymbolicCholeskyRaw::Simplicial(ghost_factorize_simplicial_symbolic(
                A,
                etree,
                col_counts,
                stack.rb_mut(),
            )?)
        };

        Ok(SymbolicCholesky {
            raw,
            perm_fwd,
            perm_inv,
            A_nnz,
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::qd::Double;
    use assert2::assert;
    use dyn_stack::GlobalPodBuffer;
    use faer_core::Mat;
    use rand::{Rng, SeedableRng};

    macro_rules! monomorphize_test {
        ($name: ident) => {
            monomorphize_test!($name, i32);
            monomorphize_test!($name, i64);
        };

        ($name: ident, $ty: ident) => {
            paste::paste! {
                #[test]
                fn [<$name _ $ty>]() {
                    $name::<$ty>();
                }
            }
        };
    }

    fn test_counts<I: Index>() {
        let truncate = I::truncate;

        let n = 11;
        let col_ptr = &[0, 3, 6, 10, 13, 16, 21, 24, 29, 31, 37, 43].map(truncate);
        let row_ind = &[
            0, 5, 6, // 0
            1, 2, 7, // 1
            1, 2, 9, 10, // 2
            3, 5, 9, // 3
            4, 7, 10, // 4
            0, 3, 5, 8, 9, // 5
            0, 6, 10, // 6
            1, 4, 7, 9, 10, // 7
            5, 8, // 8
            2, 3, 5, 7, 9, 10, // 9
            2, 4, 6, 7, 9, 10, // 10
        ]
        .map(truncate);

        let A = SymbolicSparseColMatRef::new_checked(n, n, col_ptr, None, row_ind);
        let zero = truncate(0);
        let mut etree = vec![zero; n];
        let mut col_count = vec![zero; n];
        ghost::with_size(n, |N| {
            let A = ghost::SymbolicSparseColMatRef::new(A, N, N);
            let etree = ghost_prefactorize_symbolic(
                Array::from_mut(&mut etree, N),
                Array::from_mut(&mut col_count, N),
                A,
                PodStack::new(&mut GlobalPodBuffer::new(StackReq::new::<I>(5 * n))),
            );

            ghost_factorize_supernodal_symbolic(
                A,
                etree,
                Array::from_ref(&col_count, N),
                PodStack::new(&mut GlobalPodBuffer::new(StackReq::new::<I>(20 * n))),
                Default::default(),
            )
            .unwrap();
        });
        assert_eq!(etree, [5, 2, 7, 5, 7, 6, 8, 9, 9, 10, NONE].map(truncate));
        assert_eq!(col_count, [3, 3, 4, 3, 3, 4, 4, 3, 3, 2, 1].map(truncate));
    }

    include!("../data.rs");

    fn test_amd<I: Index>() {
        for &(_, (_, col_ptr, row_ind, _)) in ALL {
            let I = I::truncate;
            let n = col_ptr.len() - 1;

            let (amd_perm, amd_perm_inv, _) =
                ::amd::order(n, col_ptr, row_ind, &Default::default()).unwrap();
            let col_ptr = &*col_ptr.iter().copied().map(I).collect::<Vec<_>>();
            let row_ind = &*row_ind.iter().copied().map(I).collect::<Vec<_>>();
            let amd_perm = &*amd_perm.iter().copied().map(I).collect::<Vec<_>>();
            let amd_perm_inv = &*amd_perm_inv.iter().copied().map(I).collect::<Vec<_>>();
            let A = SymbolicSparseColMatRef::new_checked(n, n, col_ptr, None, row_ind);

            let perm = &mut vec![I(0); n];
            let perm_inv = &mut vec![I(0); n];

            crate::amd::order_maybe_unsorted(
                perm,
                perm_inv,
                A,
                Default::default(),
                PodStack::new(&mut GlobalPodBuffer::new(
                    crate::amd::order_maybe_unsorted_req::<I>(n, row_ind.len()).unwrap(),
                )),
            )
            .unwrap();

            assert!(perm == amd_perm);
            assert!(perm_inv == amd_perm_inv);
        }
    }

    fn sparse_to_dense<I: Index, E: ComplexField>(sparse: SparseColMatRef<'_, I, E>) -> Mat<E> {
        let m = sparse.nrows();
        let n = sparse.ncols();

        let mut dense = Mat::<E>::zeros(m, n);

        for j in 0..n {
            for (i, val) in zip(
                sparse.row_indices_of_col(j),
                sparse.values_of_col(j).into_iter(),
            ) {
                dense.write(i, j, val.read());
            }
        }

        dense
    }

    fn reconstruct_from_supernodal<I: Index, E: ComplexField>(
        symbolic: &SymbolicSupernodalCholesky<I>,
        L_values: SliceGroup<'_, E>,
    ) -> Mat<E> {
        let n_supernodes = symbolic.n_supernodes();
        let n = symbolic.nrows();
        let mut dense = Mat::<E>::zeros(n, n);

        let col_ptr_row = &*symbolic.col_ptrs_for_row_indices;
        let col_ptr_val = &*symbolic.col_ptrs_for_values;
        let row_ind = &*symbolic.row_indices;

        for s in 0..n_supernodes {
            let s_start = symbolic.supernode_begin[s].zx();
            let s_end = symbolic.supernode_begin[s + 1].zx();

            let s_pattern = &row_ind[col_ptr_row[s].zx()..col_ptr_row[s + 1].zx()];
            let s_ncols = s_end - s_start;
            let s_nrows = s_pattern.len() + s_ncols;

            let Ls = MatRef::<E>::from_column_major_slice(
                L_values
                    .subslice(col_ptr_val[s].zx()..col_ptr_val[s + 1].zx())
                    .into_inner(),
                s_nrows,
                s_ncols,
            );

            let [Ls_top, Ls_bot] = Ls.split_at_row(s_ncols);
            dense
                .as_mut()
                .submatrix(s_start, s_start, s_ncols, s_ncols)
                .clone_from(Ls_top);

            for col in 0..s_ncols {
                for (i, row) in s_pattern.iter().enumerate() {
                    dense.write(row.zx(), s_start + col, Ls_bot.read(i, col));
                }
            }
        }

        let mut D = Mat::<E>::zeros(n, n);
        D.as_mut().diagonal().clone_from(dense.as_ref().diagonal());
        dense.as_mut().diagonal().fill(E::one());
        &dense * D * &dense.adjoint()
    }

    fn reconstruct_from_simplicial<I: Index, E: ComplexField>(
        symbolic: &SymbolicSimplicialCholesky<I>,
        L_values: SliceGroup<'_, E>,
    ) -> Mat<E> {
        let n = symbolic.nrows();
        let mut dense = Mat::<E>::zeros(n, n);

        let L = SparseColMatRef::new(
            SymbolicSparseColMatRef::new_checked(
                n,
                n,
                symbolic.col_ptrs(),
                None,
                symbolic.row_indices(),
            ),
            L_values,
        );

        for j in 0..n {
            for (i, val) in zip(L.row_indices_of_col(j), L.values_of_col(j).into_iter()) {
                dense.write(i, j, val.read());
            }
        }

        let mut D = Mat::<E>::zeros(n, n);
        D.as_mut().diagonal().clone_from(dense.as_ref().diagonal());
        dense.as_mut().diagonal().fill(E::one());

        &dense * D * &dense.adjoint()
    }

    fn test_supernodal<I: Index>() {
        type E = num_complex::Complex<Double<f64>>;
        let truncate = I::truncate;

        let (_, col_ptr, row_ind, values) = MEDIUM;

        let mut gen = rand::rngs::StdRng::seed_from_u64(0);

        let mut complexify = |e: E| {
            let i = E::one().neg().sqrt();
            if e == E::from_f64(1.0) {
                e.add(i.mul(E::from_f64(gen.gen())))
            } else {
                e
            }
        };

        let n = col_ptr.len() - 1;
        let nnz = values.len();
        let col_ptr = &*col_ptr.iter().copied().map(truncate).collect::<Vec<_>>();
        let row_ind = &*row_ind.iter().copied().map(truncate).collect::<Vec<_>>();
        let values_mat =
            faer_core::Mat::<E>::from_fn(nnz, 1, |i, _| complexify(E::from_f64(values[i])));
        let values = SliceGroup::new(values_mat.col_ref(0));

        let A = SparseColMatRef::<'_, I, E>::new(
            SymbolicSparseColMatRef::new_checked(n, n, col_ptr, None, row_ind),
            values,
        );
        let zero = truncate(0);
        let mut etree = vec![zero; n];
        let mut col_count = vec![zero; n];
        ghost::with_size(n, |N| {
            let A = ghost::SparseColMatRef::new(A, N, N);
            let etree = ghost_prefactorize_symbolic(
                Array::from_mut(&mut etree, N),
                Array::from_mut(&mut col_count, N),
                A.symbolic(),
                PodStack::new(&mut GlobalPodBuffer::new(StackReq::new::<I>(5 * n))),
            );

            let symbolic = ghost_factorize_supernodal_symbolic(
                A.symbolic(),
                etree,
                Array::from_ref(&col_count, N),
                PodStack::new(&mut GlobalPodBuffer::new(StackReq::new::<I>(20 * n))),
                Default::default(),
            )
            .unwrap();

            let mut A_lower_col_ptr = col_ptr.to_vec();
            let mut A_lower_values = values_mat.clone();
            let mut A_lower_row_ind = row_ind.to_vec();
            let A_lower_values = SliceGroupMut::new(A_lower_values.col_mut(0));
            let A_lower = ghost_adjoint(
                &mut A_lower_col_ptr,
                &mut A_lower_row_ind,
                A_lower_values,
                A,
                PodStack::new(&mut GlobalPodBuffer::new(StackReq::new::<I>(20 * n))),
            );
            let mut values = faer_core::Mat::<E>::zeros(symbolic.len_values(), 1);
            let mut values = SliceGroupMut::new(values.col_mut(0));

            factorize_supernodal_numeric_ldlt(
                values.rb_mut(),
                *A_lower,
                &symbolic,
                Parallelism::None,
                PodStack::new(&mut GlobalPodBuffer::new(
                    factorize_supernodal_numeric_ldlt_req::<I, E>(&symbolic, Parallelism::None)
                        .unwrap(),
                )),
            );
            let mut A = sparse_to_dense(*A);
            for j in 0..n {
                for i in j + 1..n {
                    A.write(i, j, A.read(j, i).conj());
                }
            }

            let err = reconstruct_from_supernodal(&symbolic, values.rb()) - A;
            let mut max = <E as ComplexField>::Real::zero();
            for j in 0..n {
                for i in 0..n {
                    let x = err.read(i, j).abs();
                    max = if max > x { max } else { x }
                }
            }
            assert!(max < <E as ComplexField>::Real::from_f64(1e-25));
        });
    }

    fn test_simplicial<I: Index>() {
        type E = num_complex::Complex<Double<f64>>;
        let truncate = I::truncate;

        let (_, col_ptr, row_ind, values) = SMALL;

        let complexify = |e: E| {
            let i = E::one().neg().sqrt();
            if e == E::from_f64(1.0) {
                e.add(i.mul(E::from_f64(rand::random())))
            } else {
                e
            }
        };

        let n = col_ptr.len() - 1;
        let nnz = values.len();
        let col_ptr = &*col_ptr.iter().copied().map(truncate).collect::<Vec<_>>();
        let row_ind = &*row_ind.iter().copied().map(truncate).collect::<Vec<_>>();
        let values_mat =
            faer_core::Mat::<E>::from_fn(nnz, 1, |i, _| complexify(E::from_f64(values[i])));
        let values = SliceGroup::new(values_mat.col_ref(0));

        let A = SparseColMatRef::<'_, I, E>::new(
            SymbolicSparseColMatRef::new_checked(n, n, col_ptr, None, row_ind),
            values,
        );
        let zero = truncate(0);
        let mut etree = vec![zero; n];
        let mut col_count = vec![zero; n];
        ghost::with_size(n, |N| {
            let A = ghost::SparseColMatRef::new(A, N, N);
            let etree = ghost_prefactorize_symbolic(
                Array::from_mut(&mut etree, N),
                Array::from_mut(&mut col_count, N),
                A.symbolic(),
                PodStack::new(&mut GlobalPodBuffer::new(StackReq::new::<I>(5 * n))),
            );

            let symbolic = ghost_factorize_simplicial_symbolic(
                A.symbolic(),
                etree,
                Array::from_ref(&col_count, N),
                PodStack::new(&mut GlobalPodBuffer::new(StackReq::new::<I>(20 * n))),
            )
            .unwrap();

            let mut values = faer_core::Mat::<E>::zeros(symbolic.len_values(), 1);
            let mut values = SliceGroupMut::new(values.col_mut(0));

            factorize_simplicial_numeric_ldlt(
                values.rb_mut(),
                *A,
                &symbolic,
                PodStack::new(&mut GlobalPodBuffer::new(
                    factorize_simplicial_numeric_ldlt_req::<I, E>(n).unwrap(),
                )),
            );
            let mut A = sparse_to_dense(*A);
            for j in 0..n {
                for i in j + 1..n {
                    A.write(i, j, A.read(j, i).conj());
                }
            }

            let err = reconstruct_from_simplicial(&symbolic, values.rb()) - &A;

            let mut max = <E as ComplexField>::Real::zero();
            for j in 0..n {
                for i in 0..n {
                    let x = err.read(i, j).abs();
                    max = if max > x { max } else { x }
                }
            }
            assert!(max < <E as ComplexField>::Real::from_f64(1e-25));
        });
    }

    monomorphize_test!(test_amd);
    monomorphize_test!(test_counts);
    monomorphize_test!(test_supernodal, i32);
    monomorphize_test!(test_simplicial, i32);
}
