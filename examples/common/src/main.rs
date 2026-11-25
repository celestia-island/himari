//! Common mathematical idioms: matrices, linear systems, least squares, quadratic roots, complex
//! circuits and inline macros.
//!
//! How to run: `cargo run -p himari-example-common`. Every example is compared against an
//! independent hand calculation or another numerical implementation;
//! `cargo test -p himari-example-common` replays the same set of invariants.

include!("generated.rs");

fn close(got: f64, want: f64, tolerance: f64, label: &str) {
    let scale = want.abs().max(1.0);
    assert!(
        (got - want).abs() <= tolerance * scale,
        "{label}: got={got:.17e}, want={want:.17e}"
    );
}

fn matrix_close(got: &[Vec<f64>], want: &[Vec<f64>], tolerance: f64, label: &str) {
    assert_eq!(got.len(), want.len(), "{label}: row count");
    for (i, (got_row, want_row)) in got.iter().zip(want).enumerate() {
        assert_eq!(got_row.len(), want_row.len(), "{label}: column count");
        for (j, (got, want)) in got_row.iter().zip(want_row).enumerate() {
            close(*got, *want, tolerance, &format!("{label}[{i}][{j}]"));
        }
    }
}

fn matrix_examples() {
    let theta = 0.37;
    matrix_close(
        &rotation_orthogonality(theta),
        &[vec![1.0, 0.0], vec![0.0, 1.0]],
        2e-15,
        "R' R",
    );

    let k = 2_000.0;
    matrix_close(
        &N(k),
        &[
            vec![17.0 * k * k, 7.0 * k * k],
            vec![7.0 * k * k, 10.0 * k * k],
        ],
        1e-14,
        "A' A",
    );
    matrix_close(
        &stiffness_inverse(k),
        &[
            vec![3.0 / (11.0 * k), -1.0 / (11.0 * k)],
            vec![-1.0 / (11.0 * k), 4.0 / (11.0 * k)],
        ],
        1e-14,
        "A^-1",
    );
    matrix_close(
        &stiffness_identity(k),
        &[vec![1.0, 0.0], vec![0.0, 1.0]],
        2e-15,
        "A A^-1",
    );
    matrix_close(
        &nodal_forces(k, 0.01, -0.02),
        &[
            vec![2000.0 * (4.0 * 0.01 - 0.02)],
            vec![2000.0 * (0.01 - 3.0 * 0.02)],
        ],
        1e-14,
        "A x",
    );
    println!("matrix: R'R=I, A*inv(A)=I, A*x all match independent references");
}

// Independent numerical reference: dense Gaussian elimination with partial pivoting.
// This deliberately does not share himari's symbolic elimination or generated expressions.
fn reference_solve(mut a: Vec<Vec<f64>>, mut b: Vec<f64>) -> Vec<f64> {
    let n = b.len();
    for col in 0..n {
        let pivot = (col..n)
            .max_by(|&i, &j| a[i][col].abs().total_cmp(&a[j][col].abs()))
            .unwrap();
        assert!(a[pivot][col].abs() > 1e-12, "singular reference system");
        a.swap(col, pivot);
        b.swap(col, pivot);
        for row in col + 1..n {
            let factor = a[row][col] / a[col][col];
            let (above, from_row) = a.split_at_mut(row);
            let pivot_row = &above[col];
            for (arj, acj) in from_row[0].iter_mut().zip(pivot_row).skip(col) {
                *arj -= factor * *acj;
            }
            b[row] -= factor * b[col];
        }
    }
    let mut x = vec![0.0; n];
    for i in (0..n).rev() {
        x[i] = (b[i] - (i + 1..n).map(|j| a[i][j] * x[j]).sum::<f64>()) / a[i][i];
    }
    x
}

fn spring_example() {
    let k = [1_200.0, 900.0, 1_500.0, 1_100.0];
    let f = [25.0, -10.0, 5.0];
    let got = displacements(k[0], k[1], k[2], k[3], f[0], f[1], f[2]);
    let want = reference_solve(
        vec![
            vec![k[0] + k[1], -k[1], 0.0],
            vec![-k[1], k[1] + k[2], -k[2]],
            vec![0.0, -k[2], k[2] + k[3]],
        ],
        f.to_vec(),
    );
    for (i, want) in want.iter().enumerate() {
        close(got[i][0], *want, 2e-12, &format!("spring displacement {i}"));
    }
    println!("spring system: u = {want:?} m (independent pivoted solve agrees)");
}

fn runtime_matrix_example() {
    let a = vec![
        vec![4.0, 1.0, 0.0],
        vec![1.0, 3.0, 1.0],
        vec![0.0, 1.0, 2.0],
    ];
    let b = vec![10.0, 20.0, 30.0];
    let x = runtime_displacements(&a, &b).unwrap();
    let want = reference_solve(a.clone(), b.clone());
    for (i, expected) in want.iter().enumerate() {
        close(x[i], *expected, 2e-12, "runtime LU solution");
    }
    let traction = runtime_traction(&a, &x).unwrap();
    for (i, expected) in b.iter().enumerate() {
        close(traction[i], *expected, 2e-12, "A*x must recover b");
    }
    let inverse = runtime_inverse(&a).unwrap();
    let first_inverse_column = vec![inverse[0][0], inverse[1][0], inverse[2][0]];
    let product = runtime_traction(&a, &first_inverse_column).unwrap();
    close(product[0], 1.0, 2e-12, "A*inv(A), first column");
    close(product[1], 0.0, 2e-12, "A*inv(A), first column");
    close(product[2], 0.0, 2e-12, "A*inv(A), first column");
    close(runtime_element(&a).unwrap(), 4.0, 0.0, "one-based A(1,1)");

    let epsilon = 1.0e-17;
    let tiny = runtime_displacements(&[vec![epsilon, 1.0], vec![1.0, 1.0]], &[1.0 + epsilon, 2.0])
        .unwrap();
    close(tiny[0], 1.0, 2e-15, "partial pivoting, x0");
    close(tiny[1], 1.0, 2e-15, "partial pivoting, x1");
    println!(
        "runtime matrix: solve(A,b), A*x=b, inv(A), indexing, and tiny-epsilon pivoting all pass"
    );
}

fn least_squares_example() {
    let x = [0.0, 1.0, 2.0, 3.0, 4.0];
    let y = [1.1, 2.9, 5.2, 6.8, 9.1];
    let n = x.len() as f64;
    let sum_x: f64 = x.iter().sum();
    let sum_y: f64 = y.iter().sum();
    let sum_xx: f64 = x.iter().map(|v| v * v).sum();
    let sum_xy: f64 = x.iter().zip(y).map(|(x, y)| x * y).sum();
    let got = line_fit(n, sum_x, sum_y, sum_xx, sum_xy);
    let slope = (n * sum_xy - sum_x * sum_y) / (n * sum_xx - sum_x * sum_x);
    let intercept = (sum_y - slope * sum_x) / n;
    close(got[0][0], intercept, 1e-13, "least-squares intercept");
    close(got[1][0], slope, 1e-13, "least-squares slope");
    println!("least squares: y = {intercept:.6} + {slope:.6} x");
}

fn roots_example() {
    let got = quadratic_roots(1.0, -3.0, 2.0).unwrap();
    close(got[0][0], 1.0, 1e-14, "first quadratic root");
    close(got[1][0], 2.0, 1e-14, "second quadratic root");
    let separated = quadratic_roots(1.0, -1.0e8, 1.0).unwrap();
    close(
        separated[0][0],
        1.0e-8,
        1.0e-12,
        "small stable quadratic root",
    );
    close(
        separated[1][0],
        1.0e8,
        1.0e-14,
        "large stable quadratic root",
    );
    let no_real = quadratic_roots(1.0, 0.0, 1.0).unwrap_err();
    assert!(matches!(
        no_real,
        himari::runtime::NumericError::NoRealRoots { .. }
    ));
    println!("quadratic roots: {got:?}; x^2-1e8*x+1 gives {separated:?} without cancellation");
}

fn circuit_example() {
    let (r, f, l, v) = (10.0, 1591.5494, 0.01, 100.0);
    let (re, im) = current(r, f, l, v, std::f64::consts::PI);
    let reactance = 2.0 * std::f64::consts::PI * f * l;
    let want = v / (r * r + reactance * reactance).sqrt();
    close(re.hypot(im), want, 1e-12, "AC current magnitude");
    let (mag, imaginary) = current_magnitude(r, f, l, v, std::f64::consts::PI);
    close(mag, want, 1e-12, "generated AC magnitude");
    assert_eq!(imaginary, 0.0);
    println!("AC circuit: |I|={want:.6} A with omega=2*pi*f explicit");
}

fn inline_macro_example() {
    let a = 6.0f64;
    let b = 3.0f64;
    let ratio = himari_macros::formula_matlab! {
        decl { a : length [m]; b : length [m]; }
        a / b
    };
    close(ratio, 2.0, 0.0, "inline macro ratio");
    let epsilon = 1.0e-17;
    let a = vec![vec![epsilon, 1.0], vec![1.0, 1.0]];
    let b = vec![1.0 + epsilon, 2.0];
    let x = himari_macros::formula_matlab!(
        r"decl { a : stiffness_matrix [N/m]; b : load_vector [N]; } solve(a, b)"
    )
    .unwrap();
    close(x[0], 1.0, 2e-15, "runtime macro solve, x0");
    close(x[1], 1.0, 2e-15, "runtime macro solve, x1");
    println!("inline macro: scalar formula and runtime solve(A,b) both pass");
}

fn dynamic_matrix_example() {
    let got = diffusion_stencil(0.75, 6);
    assert_eq!(got.len(), 6);
    for (i, row) in got.iter().enumerate() {
        assert_eq!(row.len(), 6);
        for (j, value) in row.iter().enumerate() {
            let want = if i == j {
                1.5
            } else if i.abs_diff(j) == 1 {
                -0.75
            } else {
                0.0
            };
            close(*value, want, 0.0, "dynamic tridiagonal stencil");
        }
    }
    println!("dynamic matrix: generated a correct 6x6 tridiagonal stencil");
}

fn main() {
    matrix_examples();
    spring_example();
    runtime_matrix_example();
    least_squares_example();
    roots_example();
    circuit_example();
    inline_macro_example();
    dynamic_matrix_example();
}

#[cfg(test)]
mod tests {
    #[test]
    fn all_common_examples_match_independent_references() {
        super::matrix_examples();
        super::spring_example();
        super::runtime_matrix_example();
        super::least_squares_example();
        super::roots_example();
        super::circuit_example();
        super::inline_macro_example();
        super::dynamic_matrix_example();
    }
}
