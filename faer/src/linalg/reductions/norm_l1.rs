use faer_traits::Unit;
use num_complex::Complex;

use super::LINEAR_IMPL_THRESHOLD;
use crate::internal_prelude::*;

#[inline(always)]
#[math]
fn norm_l1_simd<'N, C: ComplexContainer, T: ComplexField<C>>(
    ctx: &Ctx<C, T>,
    data: ColRef<'_, C, T, Dim<'N>, ContiguousFwd>,
) -> <C::Real as Container>::Of<T::RealUnit> {
    struct Impl<'a, 'N, C: ComplexContainer, T: ComplexField<C>> {
        ctx: &'a Ctx<C, T>,
        data: C::Of<&'a Array<'N, T>>,
        len: Dim<'N>,
    }

    impl<'N, C: ComplexContainer, T: ComplexField<C>> pulp::WithSimd for Impl<'_, 'N, C, T> {
        type Output = <C::Real as Container>::Of<T::RealUnit>;
        #[inline(always)]
        #[math]
        fn with_simd<S: pulp::Simd>(self, simd: S) -> Self::Output {
            let Self { ctx, data, len } = self;
            let simd = SimdCtx::<C, T, S>::new(T::simd_ctx(ctx, simd), len);

            help!(C);
            let zero = simd.splat(as_ref!(math.zero()));

            let mut acc0 = zero;
            let mut acc1 = zero;
            let mut acc2 = zero;
            let mut acc3 = zero;

            let (head, tail) = simd.batch_indices::<4>();
            for [i0, i1, i2, i3] in head {
                let x0 = simd.abs1(simd.read(rb!(data), i0));
                let x1 = simd.abs1(simd.read(rb!(data), i1));
                let x2 = simd.abs1(simd.read(rb!(data), i2));
                let x3 = simd.abs1(simd.read(rb!(data), i3));

                acc0 = simd.add(acc0, x0.0);
                acc1 = simd.add(acc1, x1.0);
                acc2 = simd.add(acc2, x2.0);
                acc3 = simd.add(acc3, x3.0);
            }

            acc0 = simd.add(acc0, acc1);
            acc2 = simd.add(acc2, acc3);
            acc0 = simd.add(acc0, acc2);

            for i0 in tail {
                let x0 = simd.abs1(simd.read(rb!(data), i0));
                acc0 = simd.add(acc0, x0.0);
            }
            if simd.has_tail() {
                let x0 = simd.abs1(simd.read_tail(rb!(data)));
                acc0 = simd.add(acc0, x0.0);
            }
            math.real(simd.reduce_sum(acc0))
        }
    }

    T::Arch::default().dispatch(Impl {
        ctx,
        data: data.as_array(),
        len: data.nrows(),
    })
}

#[math]
fn norm_l1_simd_pairwise_rows<C: ComplexContainer, T: ComplexField<C>>(
    ctx: &Ctx<C, T>,
    data: ColRef<'_, C, T, usize, ContiguousFwd>,
) -> <C::Real as Container>::Of<T::RealUnit> {
    if data.nrows() <= LINEAR_IMPL_THRESHOLD {
        with_dim!(N, data.nrows());

        norm_l1_simd(ctx, data.as_row_shape(N))
    } else {
        let split_point = ((data.nrows() + 1) / 2).next_power_of_two();
        let (head, tail) = data.split_at_row(split_point);
        let acc0 = norm_l1_simd_pairwise_rows(ctx, head);
        let acc1 = norm_l1_simd_pairwise_rows(ctx, tail);

        math.re(acc0 + acc1)
    }
}

#[math]
fn norm_l1_simd_pairwise_cols<C: ComplexContainer, T: ComplexField<C>>(
    ctx: &Ctx<C, T>,
    data: MatRef<'_, C, T, usize, usize, ContiguousFwd>,
) -> <C::Real as Container>::Of<T::RealUnit> {
    if data.ncols() == 1 {
        norm_l1_simd_pairwise_rows(ctx, data.col(0))
    } else {
        let split_point = ((data.ncols() + 1) / 2).next_power_of_two();
        let (head, tail) = data.split_at_col(split_point);
        let acc0 = norm_l1_simd_pairwise_cols(ctx, head);
        let acc1 = norm_l1_simd_pairwise_cols(ctx, tail);

        math.re(acc0 + acc1)
    }
}

#[math]
pub fn norm_l1<C: ComplexContainer, T: ComplexField<C>>(
    ctx: &Ctx<C, T>,
    mut mat: MatRef<'_, C, T>,
) -> <C::Real as Container>::Of<T::RealUnit> {
    if mat.ncols() > 1 && mat.col_stride().unsigned_abs() == 1 {
        mat = mat.transpose();
    }
    if mat.row_stride() < 0 {
        mat = mat.reverse_rows();
    }

    if mat.nrows() == 0 || mat.ncols() == 0 {
        math.re.zero()
    } else {
        let m = mat.nrows();
        let n = mat.ncols();

        if const { T::SIMD_CAPABILITIES.is_simd() } {
            if let Some(mat) = mat.try_as_col_major() {
                if const { T::IS_NATIVE_C32 } {
                    let mat: MatRef<'_, Unit, Complex<f32>, usize, usize, ContiguousFwd> =
                        unsafe { crate::hacks::coerce(mat) };
                    let mat = unsafe {
                        MatRef::<'_, Unit, f32, usize, usize, ContiguousFwd>::from_raw_parts(
                            mat.as_ptr() as *const f32,
                            2 * mat.nrows(),
                            mat.ncols(),
                            ContiguousFwd,
                            mat.col_stride().wrapping_mul(2),
                        )
                    };
                    return unsafe {
                        crate::hacks::coerce(norm_l1_simd_pairwise_cols::<Unit, f32>(
                            &Ctx(Unit),
                            mat,
                        ))
                    };
                } else if const { T::IS_NATIVE_C64 } {
                    let mat: MatRef<'_, Unit, Complex<f64>, usize, usize, ContiguousFwd> =
                        unsafe { crate::hacks::coerce(mat) };
                    let mat = unsafe {
                        MatRef::<'_, Unit, f64, usize, usize, ContiguousFwd>::from_raw_parts(
                            mat.as_ptr() as *const f64,
                            2 * mat.nrows(),
                            mat.ncols(),
                            ContiguousFwd,
                            mat.col_stride().wrapping_mul(2),
                        )
                    };
                    return unsafe {
                        crate::hacks::coerce(norm_l1_simd_pairwise_cols::<Unit, f64>(
                            &Ctx(Unit),
                            mat,
                        ))
                    };
                } else if const { C::IS_COMPLEX } {
                    let mat: MatRef<
                        num_complex::Complex<C::Real>,
                        T::RealUnit,
                        usize,
                        usize,
                        ContiguousFwd,
                    > = unsafe { crate::hacks::coerce(mat) };
                    let (re, im) = super::real_imag(mat);
                    return math.re(norm_l1_simd_pairwise_cols::<C::Real, T::RealUnit>(
                        Ctx::new(&**ctx),
                        re,
                    ) + norm_l1_simd_pairwise_cols::<C::Real, T::RealUnit>(
                        Ctx::new(&**ctx),
                        im,
                    ));
                } else {
                    return norm_l1_simd_pairwise_cols(ctx, mat);
                }
            }
        }

        let mut acc = math.re.zero();
        for j in 0..n {
            for i in 0..m {
                let val = mat.at(i, j);

                acc = math.re(acc + cx.abs1(val));
            }
        }
        acc
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{assert, unzipped, zipped, Col, Mat};

    #[test]
    fn test_norm_l1() {
        let relative_err = |a: f64, b: f64| (a - b).abs() / f64::max(a.abs(), b.abs());

        for (m, n) in [(9, 10), (1023, 5), (42, 1)] {
            for factor in [0.0, 1.0, 1e30, 1e250, 1e-30, 1e-250] {
                let mat = Mat::from_fn(m, n, |i, j| factor * ((i + j) as f64));
                let mut target = 0.0;
                zipped!(mat.as_ref()).for_each(|unzipped!(x)| {
                    target += x.abs();
                });

                if factor == 0.0 {
                    assert!(norm_l1(&default(), mat.as_ref()) == target);
                } else {
                    assert!(relative_err(norm_l1(&default(), mat.as_ref()), target) < 1e-14);
                }
            }
        }

        let mat = Col::from_fn(10000000, |_| 0.3);
        let target = 0.3 * 10000000.0f64;
        assert!(relative_err(norm_l1(&default(), mat.as_ref().as_mat()), target) < 1e-14);
    }
}