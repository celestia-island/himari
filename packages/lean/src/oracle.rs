//! The independent Rust oracle for `det3`.
//!
//! This exists so that a Lean success is never the *only* evidence: the same
//! quantity is computed twice, once by Lean's kernel via `by decide` and once
//! here in ordinary Rust integer arithmetic, and the two must agree.
//!
//! Arithmetic is exact and bounded on purpose: `i128` products of the accepted
//! literal range cannot overflow, and the multiplications are written as
//! checked operations anyway so that an overflow becomes a refusal rather than
//! a silent wrap in a release build.

/// Compute the determinant of a 3x3 matrix given in row-major order.
///
/// Uses the Leibniz formula over permutations of the row indices, which is
/// *not* the expression shape the generator writes into Lean (that one is the
/// explicit six-term Sarrus expansion). Two independently spelled versions of
/// the same identity is what makes the cross-check meaningful.
pub fn det3(values: &[i64; 9]) -> Option<i128> {
    // Row-major: `values[r * 3 + c]` is the entry at row `r`, column `c`.
    let m = |r: usize, c: usize| i128::from(values[r * 3 + c]);

    // The Leibniz formula reads `sum over sigma of sign(sigma) * prod_r m[r][sigma(r)]`.
    // Each entry is (sigma(0), sigma(1), sigma(2)) for the column indices, with
    // the sign of that permutation of `(0, 1, 2)`.
    const PERMS: [([usize; 3], i128); 6] = [
        ([0, 1, 2], 1),  // identity                (even)
        ([1, 0, 2], -1), // swap columns 0 and 1    (odd)
        ([2, 1, 0], -1), // swap columns 0 and 2    (odd)
        ([0, 2, 1], -1), // swap columns 1 and 2    (odd)
        ([1, 2, 0], 1),  // (0 1 2) -> (1 2 0)      (even)
        ([2, 0, 1], 1),  // (0 1 2) -> (2 0 1)      (even)
    ];

    let mut acc: i128 = 0;
    for (sigma, sign) in PERMS {
        // Rows are fixed, columns follow the permutation: m[0][s0]*m[1][s1]*m[2][s2].
        let a = m(0, sigma[0]);
        let b = m(1, sigma[1]);
        let c = m(2, sigma[2]);
        let term = a.checked_mul(b)?.checked_mul(c)?;
        let signed = term.checked_mul(sign)?;
        acc = acc.checked_add(signed)?;
    }
    Some(acc)
}

/// Convenience wrapper used by the runner: `None` means "the oracle refused to
/// answer", which is reported as an internal failure, never as agreement.
pub fn det3_or_fail(values: &[i64; 9]) -> Result<i128, String> {
    det3(values).ok_or_else(|| "det3 overflowed i128 for the supplied literals".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The milestone request's matrix. Its determinant is **-3**, not the -306
    /// that a first reading of the design sketch might suggest: the last entry is
    /// 10, not 9, and the two values differ by almost two orders of magnitude.
    /// The value is pinned here on purpose so that the generator, the oracle and
    /// the CLI cannot drift apart on it.
    #[test]
    fn milestone_matrix_is_minus_three() {
        assert_eq!(det3(&[1, 2, 3, 4, 5, 6, 7, 8, 10]), Some(-3));
    }

    #[test]
    fn identity_is_one_and_singular_matrices_are_zero() {
        assert_eq!(det3(&[1, 0, 0, 0, 1, 0, 0, 0, 1]), Some(1));
        assert_eq!(det3(&[0, 0, 0, 0, 0, 0, 0, 0, 0]), Some(0));
        // Two identical rows -> singular.
        assert_eq!(det3(&[1, 2, 3, 1, 2, 3, 4, 5, 6]), Some(0));
        // Two identical columns -> singular.
        assert_eq!(det3(&[1, 0, 1, 2, 0, 2, 3, 1, 3]), Some(0));
        // A row of zeros -> singular.
        assert_eq!(det3(&[1, 2, 3, 0, 0, 0, 7, 8, 9]), Some(0));
    }

    #[test]
    fn permutation_matrices_have_sign_equal_to_parity() {
        // Swap rows 1 and 2 of the identity: odd permutation.
        assert_eq!(det3(&[1, 0, 0, 0, 0, 1, 0, 1, 0]), Some(-1));
        // 3-cycle of rows: even permutation.
        assert_eq!(det3(&[0, 1, 0, 0, 0, 1, 1, 0, 0]), Some(1));
    }

    #[test]
    fn upper_triangular_determinant_is_the_product_of_the_diagonal() {
        assert_eq!(det3(&[2, 7, 9, 0, 3, 5, 0, 0, 4]), Some(24));
        assert_eq!(det3(&[-2, 7, 9, 0, -3, 5, 0, 0, -4]), Some(-24));
    }

    #[test]
    fn known_textbook_values() {
        // det [[6,1,1],[4,-2,5],[2,8,7]] = -306
        assert_eq!(det3(&[6, 1, 1, 4, -2, 5, 2, 8, 7]), Some(-306));

        // det [[2,0,1],[1,3,2],[1,1,1]] = 0
        assert_eq!(det3(&[2, 0, 1, 1, 3, 2, 1, 1, 1]), Some(0));
    }

    #[test]
    fn row_multiplication_scales_linearly() {
        let base = [2, -1, 3, 4, 5, -6, 7, 8, 9];
        let d = det3(&base).unwrap();
        let scaled = [6, -3, 9, 4, 5, -6, 7, 8, 9];
        assert_eq!(det3(&scaled), Some(3 * d));
    }

    /// The `[1,2,3,4,5,6,7,8,9]` matrix (last entry 9 instead of 10) is singular,
    /// which is the trap the milestone's 10 exists to avoid: with the 10 the same
    /// shape has the non-zero determinant -3.
    #[test]
    fn the_last_entry_carries_the_whole_difference() {
        let nine = [1, 2, 3, 4, 5, 6, 7, 8, 9];
        let ten = [1, 2, 3, 4, 5, 6, 7, 8, 10];
        assert_eq!(det3(&nine), Some(0));
        assert_eq!(det3(&ten), Some(-3));
        assert_ne!(det3(&nine), det3(&ten));
    }

    #[test]
    fn sign_flips_track_row_swaps() {
        let a = [1, 2, 3, 4, 5, 6, 7, 8, 9];
        let b = [1, 2, 3, 7, 8, 9, 4, 5, 6];
        assert_eq!(det3(&a), det3(&b));
        let c = [4, 5, 6, 1, 2, 3, 7, 8, 9];
        assert_eq!(det3(&c), det3(&a).map(|v| -v));
    }

    #[test]
    fn boundary_magnitudes_stay_inside_i128() {
        let big = 1_000_000_000_000i64;
        let m = [big, -big, big, -big, big, -big, big, -big, big];
        assert!(det3(&m).is_some());
    }

    #[test]
    fn oracle_failure_is_explicit() {
        assert_eq!(det3_or_fail(&[1, 2, 3, 4, 5, 6, 7, 8, 10]), Ok(-3));
    }
}
