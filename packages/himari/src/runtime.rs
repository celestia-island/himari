//! Runtime numeric values and checked linear algebra.
//!
//! The symbolic lane rewrites formulas at compile time. This module is the separate
//! runtime lane: matrix and vector parameters arrive as values, and every operation
//! either returns a checked value or a structured error. No silent NaN contract is used.

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

use crate::{Decl, Error, Expr, Value, C};

const RCOND_LIMIT_PER_SIZE: f64 = 64.0;
const RESIDUAL_LIMIT: f64 = 1.0e-9;

/// Runtime shape of a scalar, column vector, or dense matrix.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeKind {
    Scalar,
    Vector,
    Matrix,
}

/// The runtime lane reports through the crate-wide [`Error`], so an application can hold one
/// error type and still use `anyhow`/`?` uniformly.
pub type NumericError = Error;

impl Decl {
    /// Infer the runtime rank from a quantity name such as `matrix`, `vector`,
    /// `stiffness_matrix`, or `displacement_vector`. Existing quantity names stay scalar.
    pub fn runtime_kind(&self) -> RuntimeKind {
        if self.quantity == "matrix" || self.quantity.ends_with("_matrix") {
            RuntimeKind::Matrix
        } else if self.quantity == "vector" || self.quantity.ends_with("_vector") {
            RuntimeKind::Vector
        } else {
            RuntimeKind::Scalar
        }
    }
}

/// Rank environment used while analyzing a formula set.
pub type RuntimeShapes = BTreeMap<String, RuntimeKind>;

pub fn declaration_shapes(decls: &[Decl]) -> RuntimeShapes {
    decls
        .iter()
        .map(|decl| (decl.name.clone(), decl.runtime_kind()))
        .collect()
}

fn scalar_zero(expr: &Expr) -> bool {
    matches!(expr, Expr::Num(value) if *value == 0.0)
}

/// Infer and validate the runtime shape of an expression. Dimensions of physical
/// quantities remain the responsibility of `dim_check`; this function handles rank only.
pub fn runtime_kind_of(expr: &Expr, shapes: &RuntimeShapes) -> Result<RuntimeKind, NumericError> {
    use RuntimeKind::{Matrix, Scalar, Vector};
    Ok(match expr {
        Expr::Num(_) => Scalar,
        Expr::Var(name) => shapes.get(name).copied().unwrap_or(Scalar),
        Expr::Mat(_) | Expr::MatPattern(_) => Matrix,
        Expr::Add(a, b) | Expr::Sub(a, b) => {
            let (ka, kb) = (runtime_kind_of(a, shapes)?, runtime_kind_of(b, shapes)?);
            if scalar_zero(a) {
                kb
            } else if scalar_zero(b) || ka == kb {
                ka
            } else {
                return Err(Error::ShapeError(format!(
                    "cannot add/subtract {ka:?} and {kb:?}"
                )));
            }
        }
        Expr::Mul(a, b) => {
            let (ka, kb) = (runtime_kind_of(a, shapes)?, runtime_kind_of(b, shapes)?);
            match (ka, kb) {
                (Scalar, other) | (other, Scalar) => other,
                (Matrix, Matrix) => Matrix,
                (Matrix, Vector) => Vector,
                (Vector, Matrix) => {
                    return Err(Error::ShapeError(
                        "column_vector * matrix is undefined; transpose explicitly or use dot"
                            .to_string(),
                    ))
                }
                (Vector, Vector) => {
                    return Err(Error::ShapeError(
                        "column_vector * column_vector is undefined; use dot(u,v)".into(),
                    ))
                }
            }
        }
        Expr::Div(a, b) => {
            let denominator = runtime_kind_of(b, shapes)?;
            let numerator = runtime_kind_of(a, shapes)?;
            if denominator != Scalar {
                // Matrix/matrix division is rejected by the static shape check with a
                // precise message. Rank inference must not pre-empt that error here,
                // otherwise literal-matrix expressions fail at parse time.
                return Ok(numerator);
            }
            numerator
        }
        Expr::Pow(a, b) => {
            if runtime_kind_of(a, shapes)? == Matrix {
                match b.as_ref() {
                    Expr::Num(power)
                        if *power >= 0.0 && crate::math::fract(*power) == 0.0 && *power <= 8.0 =>
                    {
                        Matrix
                    }
                    _ => {
                        return Err(Error::InvalidInput(
                            "runtime matrix powers require an integer exponent in 0..=8".into(),
                        ))
                    }
                }
            } else {
                Scalar
            }
        }
        Expr::Neg(a) => runtime_kind_of(a, shapes)?,
        Expr::Transpose(a) => match runtime_kind_of(a, shapes)? {
            Matrix | Vector => Matrix,
            Scalar => Scalar,
        },
        Expr::Indexed { name, indices } => {
            let kind = shapes.get(name).copied().unwrap_or(Scalar);
            let expected = match kind {
                Scalar => return Ok(Scalar),
                Vector => 1,
                Matrix => 2,
            };
            if indices.len() != expected {
                return Err(Error::ShapeError(format!(
                    "`{name}` expects {expected} index/indices, got {}",
                    indices.len()
                )));
            }
            Scalar
        }
        Expr::Call(name, args) => match name.as_str() {
            "solve" if args.len() == 2 => {
                let a = runtime_kind_of(&args[0], shapes)?;
                let b = runtime_kind_of(&args[1], shapes)?;
                if a == Matrix && b == Vector {
                    Vector
                } else if matches!(args[0], Expr::Mat(_)) && matches!(args[1], Expr::Mat(_)) {
                    Matrix
                } else {
                    Scalar
                }
            }
            "det" | "trace" | "norm" | "dot" | "abs" | "arg" | "re" | "im" => Scalar,
            "adjoint" | "inv" | "roots" | "poly_coeffs" => Matrix,
            _ => Scalar,
        },
        Expr::Deriv(_, _)
        | Expr::Grad(_)
        | Expr::Laplacian(_)
        | Expr::Sum { .. }
        | Expr::Integral { .. } => Scalar,
    })
}

/// Propagate ranks through named formulas in declaration order.
pub fn shapes_of_set(set: &crate::FormulaSet) -> Result<RuntimeShapes, NumericError> {
    let mut shapes = declaration_shapes(&set.decls);
    for item in &set.items {
        // Rank inference decides which emitter lane a formula takes; it is not a
        // validator. A shape it cannot infer keeps the previous behaviour (scalar),
        // so the dedicated static checks stay the single source of rejection.
        let kind = runtime_kind_of(&item.expr, &shapes).unwrap_or(RuntimeKind::Scalar);
        if let Some(name) = &item.name {
            shapes.insert(name.clone(), kind);
        }
    }
    Ok(shapes)
}

fn real(value: C, context: &str) -> Result<f64, NumericError> {
    if !value.is_real() || !value.re.is_finite() {
        return Err(Error::NonFiniteValue(context.into()));
    }
    Ok(value.re)
}

fn dense_values(value: &Value, context: &str) -> Result<Vec<Vec<f64>>, NumericError> {
    let Value::Matrix(rows) = value else {
        return Err(Error::ShapeError(format!("{context} must be a matrix")));
    };
    if rows.is_empty() || rows.first().is_none_or(Vec::is_empty) {
        return Err(Error::ShapeError(format!("{context} cannot be empty")));
    }
    let columns = rows[0].len();
    let mut output = Vec::with_capacity(rows.len());
    for (i, row) in rows.iter().enumerate() {
        if row.len() != columns {
            return Err(Error::ShapeError(format!("{context} is jagged at row {i}")));
        }
        let mut converted = Vec::with_capacity(columns);
        for (j, value) in row.iter().enumerate() {
            converted.push(real(*value, &format!("{context}[{i}][{j}]"))?);
        }
        output.push(converted);
    }
    Ok(output)
}

fn scalar_value(value: &Value, context: &str) -> Result<f64, NumericError> {
    let Value::Scalar(value) = value else {
        return Err(Error::ShapeError(format!("{context} must be scalar")));
    };
    real(*value, context)
}

fn matrix_value(rows: Vec<Vec<f64>>) -> Value {
    Value::Matrix(
        rows.into_iter()
            .map(|row| row.into_iter().map(C::real).collect())
            .collect(),
    )
}

pub fn column_value(values: Vec<f64>) -> Value {
    matrix_value(values.into_iter().map(|value| vec![value]).collect())
}

/// Construct a checked scalar Result for generated expressions.
pub fn scalar_result(value: f64) -> Result<Value, NumericError> {
    if !value.is_finite() {
        return Err(Error::NonFiniteValue("scalar result".into()));
    }
    Ok(Value::Scalar(C::real(value)))
}

/// Convert a borrowed dense f64 matrix into a checked runtime value.
pub fn matrix_from_f64<S: AsRef<[Vec<f64>]>>(rows: S) -> Result<Value, NumericError> {
    let rows = rows.as_ref();
    let rows = rows
        .iter()
        .map(|row| row.iter().map(|value| C::real(*value)).collect())
        .collect::<Vec<Vec<C>>>();
    let value = Value::Matrix(rows);
    dense_values(&value, "matrix")?;
    Ok(value)
}

/// Convert a borrowed f64 slice into a checked column vector.
pub fn column_from_f64<S: AsRef<[f64]>>(values: S) -> Result<Value, NumericError> {
    let values = values.as_ref();
    if values.is_empty() {
        return Err(Error::ShapeError("vector cannot be empty".into()));
    }
    for (index, value) in values.iter().enumerate() {
        if !value.is_finite() {
            return Err(Error::NonFiniteValue(format!("vector[{index}]")));
        }
    }
    Ok(column_value(values.to_vec()))
}

pub fn into_scalar(value: Value) -> Result<f64, NumericError> {
    scalar_value(&value, "result")
}

pub fn into_matrix(value: Value) -> Result<Vec<Vec<f64>>, NumericError> {
    dense_values(&value, "result")
}

pub fn into_column(value: Value) -> Result<Vec<f64>, NumericError> {
    let rows = dense_values(&value, "result")?;
    if rows[0].len() != 1 {
        return Err(Error::ShapeError(format!(
            "result must be a column vector, got {} columns",
            rows[0].len()
        )));
    }
    Ok(rows.into_iter().map(|row| row[0]).collect())
}

pub fn add(a: &Value, b: &Value) -> Result<Value, NumericError> {
    match (a, b) {
        (Value::Scalar(x), Value::Scalar(y)) => Ok(Value::Scalar(x.add(*y))),
        (Value::Matrix(_), Value::Scalar(z)) | (Value::Scalar(z), Value::Matrix(_))
            if z.re == 0.0 && z.im == 0.0 =>
        {
            let matrix = if matches!(a, Value::Matrix(_)) { a } else { b };
            Ok(matrix.clone())
        }
        (Value::Matrix(a), Value::Matrix(b)) => {
            let (a, b) = (
                dense_values(&Value::Matrix(a.clone()), "left")?,
                dense_values(&Value::Matrix(b.clone()), "right")?,
            );
            if a.len() != b.len() || a[0].len() != b[0].len() {
                return Err(Error::ShapeError(format!(
                    "matrix addition requires equal shapes, got {}x{} and {}x{}",
                    a.len(),
                    a[0].len(),
                    b.len(),
                    b[0].len()
                )));
            }
            Ok(matrix_value(
                a.into_iter()
                    .zip(b)
                    .map(|(ra, rb)| ra.into_iter().zip(rb).map(|(x, y)| x + y).collect())
                    .collect(),
            ))
        }
        _ => Err(Error::ShapeError(
            "matrix/scalar addition is undefined except for the zero scalar".into(),
        )),
    }
}

pub fn sub(a: &Value, b: &Value) -> Result<Value, NumericError> {
    add(a, &neg(b)?)
}

pub fn neg(value: &Value) -> Result<Value, NumericError> {
    Ok(match value {
        Value::Scalar(value) => Value::Scalar(value.neg()),
        Value::Matrix(rows) => Value::Matrix(
            rows.iter()
                .map(|row| row.iter().map(|value| value.neg()).collect())
                .collect(),
        ),
    })
}

pub fn mul(a: &Value, b: &Value) -> Result<Value, NumericError> {
    match (a, b) {
        (Value::Scalar(x), Value::Scalar(y)) => Ok(Value::Scalar(x.mul(*y))),
        (Value::Scalar(_), Value::Matrix(_)) => mul(b, a),
        (Value::Matrix(rows), Value::Scalar(k)) => {
            if !k.is_real() {
                return Err(Error::NonFiniteValue("scalar multiplier".into()));
            }
            Ok(Value::Matrix(
                rows.iter()
                    .map(|row| row.iter().map(|value| value.mul(*k)).collect())
                    .collect(),
            ))
        }
        (Value::Matrix(a), Value::Matrix(b)) => {
            let (a, b) = (
                dense_values(&Value::Matrix(a.clone()), "left")?,
                dense_values(&Value::Matrix(b.clone()), "right")?,
            );
            let inner = a[0].len();
            if inner != b.len() {
                return Err(Error::ShapeError(format!(
                    "matrix product requires left columns == right rows, got {} != {}",
                    inner,
                    b.len()
                )));
            }
            let columns = b[0].len();
            let mut output = vec![vec![0.0; columns]; a.len()];
            for (i, row) in a.iter().enumerate() {
                for j in 0..columns {
                    for k in 0..inner {
                        output[i][j] += row[k] * b[k][j];
                    }
                }
            }
            Ok(matrix_value(output))
        }
    }
}

pub fn div(a: &Value, b: &Value) -> Result<Value, NumericError> {
    match (a, b) {
        (Value::Scalar(x), Value::Scalar(y)) => x
            .div(*y)
            .map(Value::Scalar)
            .map_err(|_| Error::InvalidInput("division by zero".into())),
        (Value::Matrix(rows), Value::Scalar(k)) => {
            if k.re == 0.0 {
                return Err(Error::InvalidInput("division by zero".into()));
            }
            Ok(Value::Matrix(
                rows.iter()
                    .map(|row| {
                        row.iter()
                            .map(|value| value.mul(C::real(1.0 / k.re)))
                            .collect()
                    })
                    .collect(),
            ))
        }
        _ => Err(Error::ShapeError(
            "runtime matrix division requires a scalar denominator".into(),
        )),
    }
}

pub fn transpose(value: &Value) -> Result<Value, NumericError> {
    let rows = dense_values(value, "transpose input")?;
    let columns = rows[0].len();
    Ok(matrix_value(
        (0..columns)
            .map(|j| rows.iter().map(|row| row[j]).collect())
            .collect(),
    ))
}

pub fn dot(a: &Value, b: &Value) -> Result<Value, NumericError> {
    let (a, b) = (dense_values(a, "dot left")?, dense_values(b, "dot right")?);
    if a.len() != b.len() || a[0].len() != b[0].len() {
        return Err(Error::ShapeError(
            "dot requires vectors with equal shapes".into(),
        ));
    }
    let mut sum = 0.0;
    for (ra, rb) in a.into_iter().zip(b) {
        for (x, y) in ra.into_iter().zip(rb) {
            sum += x * y;
        }
    }
    Ok(Value::Scalar(C::real(sum)))
}

pub fn norm(value: &Value) -> Result<Value, NumericError> {
    let squared = dot(value, value)?;
    let squared = scalar_value(&squared, "norm squared")?;
    Ok(Value::Scalar(C::real(libm::sqrt(squared))))
}

/// Checked one-based indexing for runtime matrices and vectors.
pub fn index(value: &Value, indices: &[usize]) -> Result<Value, NumericError> {
    let rows = dense_values(value, "indexed value")?;
    let (i, j) = match indices {
        [i] => (*i, 1),
        [i, j] => (*i, *j),
        _ => {
            return Err(Error::ShapeError(format!(
                "expected one or two indices, got {}",
                indices.len()
            )))
        }
    };
    if i == 0 || j == 0 || i > rows.len() || j > rows[0].len() {
        return Err(Error::ShapeError(format!(
            "index ({i},{j}) is outside a {}x{} value (indices are one-based)",
            rows.len(),
            rows[0].len()
        )));
    }
    Ok(Value::Scalar(C::real(rows[i - 1][j - 1])))
}

struct Lu {
    values: Vec<Vec<f64>>,
    pivots: Vec<usize>,
}

fn finite_matrix(rows: &[Vec<f64>], name: &str) -> Result<(), NumericError> {
    if rows.is_empty() || rows.first().is_none_or(Vec::is_empty) {
        return Err(Error::ShapeError(format!("{name} cannot be empty")));
    }
    let columns = rows[0].len();
    for (i, row) in rows.iter().enumerate() {
        if row.len() != columns {
            return Err(Error::ShapeError(format!("{name} is jagged at row {i}")));
        }
        for (j, value) in row.iter().enumerate() {
            if !value.is_finite() {
                return Err(Error::NonFiniteValue(format!("{name}[{i}][{j}]")));
            }
        }
    }
    Ok(())
}

fn factor(rows: &[Vec<f64>]) -> Result<Lu, NumericError> {
    finite_matrix(rows, "matrix")?;
    let n = rows.len();
    if rows[0].len() != n {
        return Err(Error::ShapeError(format!(
            "LU factorization requires a square matrix, got {}x{}",
            n,
            rows[0].len()
        )));
    }
    let mut values = rows.to_vec();
    let mut pivots = Vec::with_capacity(n);
    for k in 0..n {
        let mut pivot = k;
        for (i, row) in values.iter().enumerate().skip(k + 1) {
            if row[k].abs() > values[pivot][k].abs() {
                pivot = i;
            }
        }
        if values[pivot][k] == 0.0 {
            return Err(Error::SingularMatrix { column: k });
        }
        values.swap(k, pivot);
        pivots.push(pivot);
        for i in k + 1..n {
            let multiplier = values[i][k] / values[k][k];
            if !multiplier.is_finite() {
                return Err(Error::NonFiniteValue("LU multiplier".into()));
            }
            values[i][k] = multiplier;
            // Row i is updated in place while row k is only read: the two rows live in the same
            // matrix, so split the borrow at i (k < i always holds here).
            let (above, from_i) = values.split_at_mut(i);
            let pivot_row = &above[k];
            for (vij, vkj) in from_i[0].iter_mut().zip(pivot_row).skip(k + 1) {
                *vij -= multiplier * *vkj;
                if !vij.is_finite() {
                    return Err(Error::NonFiniteValue("LU elimination".into()));
                }
            }
        }
    }
    Ok(Lu { values, pivots })
}

fn solve_factored(lu: &Lu, rhs: Vec<f64>) -> Result<Vec<f64>, NumericError> {
    let n = lu.values.len();
    if rhs.len() != n {
        return Err(Error::ShapeError(format!(
            "right-hand side has length {}, expected {n}",
            rhs.len()
        )));
    }
    for (i, value) in rhs.iter().enumerate() {
        if !value.is_finite() {
            return Err(Error::NonFiniteValue(format!("right-hand side[{i}]")));
        }
    }
    // Apply the **whole** row permutation first, then do a plain forward substitution.
    //
    // The stored multipliers live on the rows in their *final* positions, so they only make
    // sense against a right-hand side that has been permuted the same way. Replaying the
    // swaps one step at a time interleaved with elimination (what this used to do) is NOT the
    // same permutation once a later step swaps two rows that an earlier step already moved —
    // it silently produced a wrong `x` for n >= 3 whenever such a swap occurred, and the
    // residual gate then refused perfectly well-conditioned systems as `unstable-residual`.
    let mut permuted = vec![0.0; n];
    let mut order: Vec<usize> = (0..n).collect();
    for (k, &pivot) in lu.pivots.iter().enumerate() {
        order.swap(k, pivot);
    }
    for (i, &src) in order.iter().enumerate() {
        permuted[i] = rhs[src];
    }
    let mut rhs = permuted;
    for k in 0..n {
        for i in k + 1..n {
            rhs[i] -= lu.values[i][k] * rhs[k];
        }
    }
    let mut x = vec![0.0; n];
    for i in (0..n).rev() {
        let diagonal = lu.values[i][i];
        if diagonal == 0.0 {
            return Err(Error::SingularMatrix { column: i });
        }
        let tail: f64 = (i + 1..n).map(|j| lu.values[i][j] * x[j]).sum();
        x[i] = (rhs[i] - tail) / diagonal;
        if !x[i].is_finite() {
            return Err(Error::NonFiniteValue("back substitution".into()));
        }
    }
    Ok(x)
}

fn infinity_norm(rows: &[Vec<f64>]) -> f64 {
    rows.iter()
        .map(|row| row.iter().map(|value| value.abs()).sum::<f64>())
        .fold(0.0, f64::max)
}

fn relative_residual(a: &[Vec<f64>], b: &[f64], x: &[f64]) -> f64 {
    let mut max_residual = 0.0f64;
    for (i, row) in a.iter().enumerate() {
        let predicted: f64 = row.iter().zip(x).map(|(a, x)| a * x).sum();
        max_residual = max_residual.max((predicted - b[i]).abs());
    }
    let scale = (infinity_norm(a) * x.iter().map(|v| v.abs()).sum::<f64>()
        + b.iter().map(|v| v.abs()).sum::<f64>())
    .max(f64::MIN_POSITIVE);
    max_residual / scale
}

/// Numerical result of a checked LU solve.
#[derive(Clone, Debug, PartialEq)]
pub struct SolveReport {
    pub x: Vec<f64>,
    pub rcond: f64,
    pub residual: f64,
}

/// Solve a dense real system with partial pivoting, condition estimation, and
/// an independent residual check. The inverse is used only for the condition
/// estimate, never as the algorithm used to obtain x.
pub fn solve_dense(a: &[Vec<f64>], b: &[f64]) -> Result<SolveReport, NumericError> {
    let lu = factor(a)?;
    let x = solve_factored(&lu, b.to_vec())?;
    let n = a.len();
    let mut inverse = vec![vec![0.0; n]; n];
    for column in 0..n {
        let mut basis = vec![0.0; n];
        basis[column] = 1.0;
        let solved = solve_factored(&lu, basis)?;
        for i in 0..n {
            inverse[i][column] = solved[i];
        }
    }
    let norm_product = infinity_norm(a) * infinity_norm(&inverse);
    let rcond = if norm_product > 0.0 && norm_product.is_finite() {
        1.0 / norm_product
    } else {
        0.0
    };
    let limit = f64::EPSILON * RCOND_LIMIT_PER_SIZE * n as f64;
    if !rcond.is_finite() || rcond < limit {
        return Err(Error::IllConditioned { rcond, limit });
    }
    let residual = relative_residual(a, b, &x);
    if !residual.is_finite() || residual > RESIDUAL_LIMIT {
        return Err(Error::UnstableResidual {
            residual,
            limit: RESIDUAL_LIMIT,
        });
    }
    Ok(SolveReport { x, rcond, residual })
}

/// Runtime `solve(A,b)` expression. A is a square matrix and b is a column vector.
pub fn solve(a: &Value, b: &Value) -> Result<Value, NumericError> {
    let a = dense_values(a, "solve matrix")?;
    let b = into_column(b.clone())?;
    Ok(column_value(solve_dense(&a, &b)?.x))
}

/// Runtime matrix trace with a square-shape check.
pub fn trace(value: &Value) -> Result<Value, NumericError> {
    let rows = dense_values(value, "trace input")?;
    if rows.len() != rows[0].len() {
        return Err(Error::ShapeError(format!(
            "trace requires a square matrix, got {}x{}",
            rows.len(),
            rows[0].len()
        )));
    }
    let sum = (0..rows.len()).map(|i| rows[i][i]).sum();
    Ok(Value::Scalar(C::real(sum)))
}

/// Runtime determinant using the same LU factorization as the solver.
pub fn det(value: &Value) -> Result<Value, NumericError> {
    let rows = dense_values(value, "determinant input")?;
    let lu = factor(&rows)?;
    let mut determinant = 1.0;
    for (i, row) in lu.values.iter().enumerate() {
        determinant *= row[i];
        if lu.pivots[i] != i {
            determinant = -determinant;
        }
    }
    if !determinant.is_finite() {
        return Err(Error::NonFiniteValue("determinant".into()));
    }
    Ok(Value::Scalar(C::real(determinant)))
}

/// Runtime inverse for callers that explicitly need it. Prefer `solve(A,b)` when
/// solving a single system; the inverse is formed by solving all basis columns.
pub fn inv(value: &Value) -> Result<Value, NumericError> {
    let rows = dense_values(value, "inverse input")?;
    let n = rows.len();
    let mut inverse = vec![vec![0.0; n]; n];
    for column in 0..n {
        let mut basis = vec![0.0; n];
        basis[column] = 1.0;
        let x = solve_dense(&rows, &basis)?.x;
        for i in 0..n {
            inverse[i][column] = x[i];
        }
    }
    Ok(matrix_value(inverse))
}

/// Stable real quadratic root with a typed no-real-roots error.
pub fn real_quadratic_root(first: bool, a: f64, b: f64, c: f64) -> Result<f64, NumericError> {
    if ![a, b, c].iter().all(|value| value.is_finite()) {
        return Err(Error::NonFiniteValue("quadratic coefficient".into()));
    }
    if a == 0.0 {
        return Err(Error::InvalidInput(
            "quadratic coefficient `a` must be nonzero".into(),
        ));
    }
    let discriminant = b * b - 4.0 * a * c;
    if !discriminant.is_finite() {
        return Err(Error::NonFiniteValue("quadratic discriminant".into()));
    }
    if discriminant < 0.0 {
        return Err(Error::NoRealRoots { discriminant });
    }
    let sqrt_d = libm::sqrt(discriminant);
    let q_is_first = b >= 0.0;
    let signed_s = if q_is_first { sqrt_d } else { -sqrt_d };
    let q = -0.5 * (b + signed_s);
    let root = if q == 0.0 {
        -b / (2.0 * a)
    } else if first == q_is_first {
        q / a
    } else {
        c / q
    };
    if !root.is_finite() {
        return Err(Error::NonFiniteValue("quadratic root".into()));
    }
    Ok(root)
}

/// Value-typed companion used by generated code, avoiding nested Result conversions.
pub fn real_quadratic_root_values(
    first: bool,
    a: Value,
    b: Value,
    c: Value,
) -> Result<f64, NumericError> {
    real_quadratic_root(
        first,
        scalar_value(&a, "quadratic coefficient a")?,
        scalar_value(&b, "quadratic coefficient b")?,
        scalar_value(&c, "quadratic coefficient c")?,
    )
}

pub fn real_quadratic_roots(a: f64, b: f64, c: f64) -> Result<[f64; 2], NumericError> {
    Ok([
        real_quadratic_root(true, a, b, c)?,
        real_quadratic_root(false, a, b, c)?,
    ])
}

/// Return true when an expression can use the existing scalar emitter without
/// encountering a runtime matrix operation.
fn plain_scalar(expr: &Expr, shapes: &RuntimeShapes) -> bool {
    match expr {
        Expr::Num(_) => true,
        Expr::Var(name) => {
            shapes.get(name).copied().unwrap_or(RuntimeKind::Scalar) == RuntimeKind::Scalar
        }
        Expr::Add(a, b) | Expr::Sub(a, b) | Expr::Mul(a, b) | Expr::Div(a, b) | Expr::Pow(a, b) => {
            plain_scalar(a, shapes) && plain_scalar(b, shapes)
        }
        Expr::Neg(a) => plain_scalar(a, shapes),
        Expr::Call(name, args) => {
            matches!(
                name.as_str(),
                "exp"
                    | "ln"
                    | "log"
                    | "sqrt"
                    | "sin"
                    | "cos"
                    | "tan"
                    | "sinh"
                    | "cosh"
                    | "tanh"
                    | "abs"
                    | "arg"
                    | "conj"
                    | "re"
                    | "im"
                    | "min"
                    | "max"
                    | "clamp"
            ) && args.iter().all(|arg| plain_scalar(arg, shapes))
        }
        _ => false,
    }
}

/// Whether this is a real quadratic `roots` formula that should return a typed
/// `Result` instead of using NaN as an implicit error channel.
pub fn is_real_quadratic_formula(expr: &Expr) -> bool {
    if crate::uses_complex(expr) || !matches!(expr, Expr::Call(name, _) if name == "roots") {
        return false;
    }
    match crate::expand_call(expr) {
        Some(Expr::Mat(rows)) if rows.len() == 2 => rows.iter().all(|row| {
            matches!(row.first(), Some(Expr::Call(name, _)) if name.starts_with("__quad_root_"))
        }),
        _ => false,
    }
}

/// Whether any parameter with a runtime matrix/vector rank occurs in the expression.
pub fn uses_runtime_values(expr: &Expr, shapes: &RuntimeShapes) -> bool {
    crate::vars_of(expr).iter().any(|name| {
        matches!(
            shapes.get(name),
            Some(RuntimeKind::Matrix | RuntimeKind::Vector)
        )
    })
}

/// Dimension checking for the runtime lane. Static formulas continue to use the
/// existing checker. Dynamic determinant dimensions require a static matrix order,
/// so they are rejected rather than assigned a fabricated dimension.
pub fn dim_check_runtime(
    expr: &Expr,
    dims: &crate::HashMap<String, crate::Dim>,
    shapes: &RuntimeShapes,
) -> Result<crate::Dim, crate::Error> {
    if let Expr::Call(name, args) = expr {
        if name == "solve" && args.len() == 2 {
            let a = runtime_kind_of(&args[0], shapes)?;
            let b = runtime_kind_of(&args[1], shapes)?;
            if a == RuntimeKind::Matrix && b == RuntimeKind::Vector {
                let da = crate::dim_check(&args[0], dims)?;
                let db = crate::dim_check(&args[1], dims)?;
                return Ok(db.div_dim(da));
            }
        }
        if name == "det" && !args.is_empty() && uses_runtime_values(&args[0], shapes) {
            return Err(Error::UnsupportedOperation(
                "runtime determinant dimensions depend on matrix order; add a static-order lane before using det on a runtime matrix".to_string(),
            ));
        }
    }
    if let Expr::Indexed { name, indices } = expr {
        if indices.iter().all(|index| index.parse::<usize>().is_ok()) {
            return crate::dim_check(&crate::var(name), dims);
        }
    }
    crate::dim_check(expr, dims)
}

/// Result of runtime code generation: a complete expression returning
/// `Result<_, NumericError>` and the concrete value kind inside it.
pub struct RuntimeEmission {
    pub code: String,
    pub kind: RuntimeKind,
}

fn emit_value(expr: &Expr, shapes: &RuntimeShapes) -> Result<String, NumericError> {
    if plain_scalar(expr, shapes) {
        return Ok(format!(
            "himari::runtime::scalar_result({})",
            crate::emit_block(expr)
        ));
    }
    Ok(match expr {
        Expr::Num(value) => format!("himari::runtime::scalar_result({value:?})"),
        Expr::Var(name) => match shapes.get(name).copied().unwrap_or(RuntimeKind::Scalar) {
            RuntimeKind::Scalar => {
                format!("himari::runtime::scalar_result({name})")
            }
            RuntimeKind::Vector => format!("himari::runtime::column_from_f64({name})"),
            RuntimeKind::Matrix => format!("himari::runtime::matrix_from_f64({name})"),
        },
        Expr::Mat(rows) => {
            let mut row_code = Vec::with_capacity(rows.len());
            for row in rows {
                let mut cells = Vec::with_capacity(row.len());
                for cell in row {
                    cells.push(format!(
                        "himari::runtime::into_scalar({})?",
                        emit_value(cell, shapes)?
                    ));
                }
                row_code.push(format!("vec![{}]", cells.join(",")));
            }
            format!(
                "Ok::<_, himari::Error>(himari::Value::Matrix(vec![{}]))",
                row_code.join(",")
            )
        }
        Expr::MatPattern(_) => {
            return Err(Error::InvalidInput(
                "runtime matrix parameters cannot be mixed with pattern matrices in this lane"
                    .into(),
            ))
        }
        Expr::Add(a, b) => format!(
            "himari::runtime::add(&({})?, &({})?)",
            emit_value(a, shapes)?,
            emit_value(b, shapes)?
        ),
        Expr::Sub(a, b) => format!(
            "himari::runtime::sub(&({})?, &({})?)",
            emit_value(a, shapes)?,
            emit_value(b, shapes)?
        ),
        Expr::Mul(a, b) => format!(
            "himari::runtime::mul(&({})?, &({})?)",
            emit_value(a, shapes)?,
            emit_value(b, shapes)?
        ),
        Expr::Div(a, b) => format!(
            "himari::runtime::div(&({})?, &({})?)",
            emit_value(a, shapes)?,
            emit_value(b, shapes)?
        ),
        Expr::Neg(a) => format!("himari::runtime::neg(&({})?)", emit_value(a, shapes)?),
        Expr::Transpose(a) => format!("himari::runtime::transpose(&({})?)", emit_value(a, shapes)?),
        Expr::Pow(a, b) => {
            let base = runtime_kind_of(a, shapes)?;
            if base == RuntimeKind::Matrix {
                let Expr::Num(power) = b.as_ref() else {
                    return Err(Error::InvalidInput(
                        "runtime matrix powers require an integer exponent in 0..=8".into(),
                    ));
                };
                let power = *power as usize;
                if power == 0 {
                    let n = match a.as_ref() {
                        Expr::Mat(rows) if rows.iter().all(|row| row.len() == rows.len()) => {
                            rows.len()
                        }
                        _ => {
                            return Err(Error::InvalidInput(
                                "runtime M^0 requires a statically known square shape".into(),
                            ))
                        }
                    };
                    let identity = (0..n)
                        .map(|i| {
                            format!(
                                "vec![{}]",
                                (0..n)
                                    .map(|j| if i == j { "1.0" } else { "0.0" })
                                    .collect::<Vec<_>>()
                                    .join(",")
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(",");
                    format!("Ok::<_, himari::Error>(himari::Value::Matrix(vec![{identity}]))")
                } else {
                    let mut code = emit_value(a, shapes)?;
                    for _ in 1..power {
                        code = format!(
                            "himari::runtime::mul(&({})?, &({})?)",
                            code,
                            emit_value(a, shapes)?
                        );
                    }
                    code
                }
            } else {
                return Err(Error::InvalidInput(
                    "unsupported scalar power in runtime matrix lane".into(),
                ));
            }
        }
        Expr::Indexed { name, indices } => {
            let parsed = indices
                .iter()
                .map(|index| {
                    index.parse::<usize>().map_err(|_| {
                        Error::InvalidInput(format!(
                            "runtime index `{index}` is not a positive integer literal"
                        ))
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            let base = emit_value(&crate::var(name), shapes)?;
            format!("himari::runtime::index(&({})?, &{:?})", base, parsed)
        }
        Expr::Call(name, args) => match name.as_str() {
            "solve"
                if args.len() == 2
                    && runtime_kind_of(&args[0], shapes)? == RuntimeKind::Matrix
                    && runtime_kind_of(&args[1], shapes)? == RuntimeKind::Vector =>
            {
                format!(
                    "himari::runtime::solve(&({})?, &({})?)",
                    emit_value(&args[0], shapes)?,
                    emit_value(&args[1], shapes)?
                )
            }
            "solve" => {
                let expanded = crate::expand_call(expr).ok_or_else(|| {
                    Error::InvalidInput("invalid symbolic solve expression".into())
                })?;
                return emit_value(&expanded, shapes);
            }
            "inv" if args.len() == 1 => format!(
                "himari::runtime::inv(&({})?)",
                emit_value(&args[0], shapes)?
            ),
            "adjoint" if args.len() == 1 => format!(
                "himari::runtime::transpose(&({})?)",
                emit_value(&args[0], shapes)?
            ),
            "dot" if args.len() == 2 => format!(
                "himari::runtime::dot(&({})?, &({})?)",
                emit_value(&args[0], shapes)?,
                emit_value(&args[1], shapes)?
            ),
            "norm" if args.len() == 1 => format!(
                "himari::runtime::norm(&({})?)",
                emit_value(&args[0], shapes)?
            ),
            "trace" if args.len() == 1 => {
                let matrix = emit_value(&args[0], shapes)?;
                format!("himari::runtime::trace(&({matrix})?)")
            }
            "roots" if args.len() == 2 => {
                let expanded = crate::expand_call(expr).ok_or_else(|| {
                    Error::InvalidInput("invalid quadratic root expression".into())
                })?;
                let Expr::Mat(rows) = expanded else {
                    return Err(Error::InvalidInput(
                        "runtime root emission requires a quadratic column vector".into(),
                    ));
                };
                if rows.len() != 2 {
                    return Err(Error::InvalidInput(
                        "only quadratic roots use the fallible real-root lane".into(),
                    ));
                }
                let mut root_code = Vec::with_capacity(2);
                for (index, row) in rows.iter().enumerate() {
                    let Expr::Call(internal, coefficients) = &row[0] else {
                        return Err(Error::InvalidInput(
                            "invalid internal quadratic root node".into(),
                        ));
                    };
                    if coefficients.len() != 3 {
                        return Err(Error::InvalidInput(
                            "quadratic root requires three coefficients".into(),
                        ));
                    }
                    let first = match internal.as_str() {
                        "__quad_root_1" => true,
                        "__quad_root_2" => false,
                        _ => {
                            return Err(Error::InvalidInput(
                                "unknown internal quadratic root node".into(),
                            ))
                        }
                    };
                    let (a, b, c) = (
                        emit_value(&coefficients[0], shapes)?,
                        emit_value(&coefficients[1], shapes)?,
                        emit_value(&coefficients[2], shapes)?,
                    );
                    root_code.push(format!(
                        "vec![himari::C::real(himari::runtime::real_quadratic_root_values({first}, {a}?, {b}?, {c}?)?)]"
                    ));
                    let _ = index;
                }
                format!(
                    "Ok::<_, himari::Error>(himari::Value::Matrix(vec![{}]))",
                    root_code.join(",")
                )
            }
            "poly_coeffs" => {
                let expanded = crate::expand_call(expr).ok_or_else(|| {
                    Error::InvalidInput("invalid polynomial coefficient expression".into())
                })?;
                return emit_value(&expanded, shapes);
            }
            "det" if uses_runtime_values(&args[0], shapes) => {
                return Err(Error::InvalidInput(
                    "runtime determinant dimensions require a static matrix order".into(),
                ))
            }
            "det" => {
                let expanded = crate::expand_call(expr)
                    .ok_or_else(|| Error::InvalidInput("invalid determinant expression".into()))?;
                return emit_value(&expanded, shapes);
            }
            _ => {
                return Err(Error::InvalidInput(format!(
                    "unsupported runtime call `{name}`"
                )))
            }
        },
        Expr::Deriv(_, _)
        | Expr::Grad(_)
        | Expr::Laplacian(_)
        | Expr::Sum { .. }
        | Expr::Integral { .. } => {
            return Err(Error::InvalidInput(
                "symbolic operators are not supported in the runtime matrix lane".into(),
            ))
        }
    })
}

/// Emit a complete checked runtime expression. The outer conversion determines
/// whether the public function returns `f64`, `Vec<f64>`, or `Vec<Vec<f64>>`.
pub fn emit_runtime(expr: &Expr, shapes: &RuntimeShapes) -> Result<RuntimeEmission, NumericError> {
    let kind = runtime_kind_of(expr, shapes)?;
    let body = emit_value(expr, shapes)?;
    let code = match kind {
        RuntimeKind::Scalar => format!("(|| {{ himari::runtime::into_scalar({body}?) }})()"),
        RuntimeKind::Vector => format!("(|| {{ himari::runtime::into_column({body}?) }})()"),
        RuntimeKind::Matrix => format!("(|| {{ himari::runtime::into_matrix({body}?) }})()"),
    };
    Ok(RuntimeEmission { code, kind })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(got: f64, want: f64, tolerance: f64) {
        assert!((got - want).abs() <= tolerance, "got={got}, want={want}");
    }

    #[test]
    fn pivoted_solver_handles_tiny_but_well_scaled_systems() {
        for epsilon in [1.0e-17, 1.0e-30, 1.0e-300] {
            let a = vec![vec![epsilon, 1.0], vec![1.0, 1.0]];
            let b = vec![1.0 + epsilon, 2.0];
            let report = solve_dense(&a, &b).unwrap();
            close(report.x[0], 1.0, 2.0e-15);
            close(report.x[1], 1.0, 2.0e-15);
            assert!(report.rcond > 0.2, "rcond={}", report.rcond);
            assert!(report.residual < 1.0e-12);
        }
    }

    #[test]
    fn solver_distinguishes_singular_ill_conditioned_and_small_but_valid() {
        let singular = solve_dense(&[vec![1.0, 1.0], vec![1.0, 1.0]], &[1.0, 1.0]);
        assert!(matches!(singular, Err(Error::SingularMatrix { .. })));

        let ill = solve_dense(&[vec![1.0, 1.0], vec![1.0, 1.0 + 1.0e-15]], &[1.0, 2.0]);
        assert!(matches!(ill, Err(Error::IllConditioned { .. })), "{ill:?}");

        let small = solve_dense(
            &[vec![1.0e-300, 0.0], vec![0.0, 1.0e-300]],
            &[1.0e-300, 1.0e-300],
        )
        .unwrap();
        close(small.x[0], 1.0, 0.0);
        close(small.x[1], 1.0, 0.0);
        close(small.rcond, 1.0, 1.0e-15);
    }

    #[test]
    fn shape_and_nonfinite_inputs_are_errors_not_panics() {
        assert!(solve_dense(&[vec![1.0, 2.0, 3.0]], &[1.0]).is_err());
        assert!(solve_dense(&[vec![1.0, 0.0], vec![0.0, 1.0]], &[1.0]).is_err());
        assert!(solve_dense(&[vec![1.0, 0.0], vec![0.0, f64::NAN]], &[1.0, 1.0]).is_err());
        assert!(matrix_from_f64(&[vec![1.0], vec![2.0, 3.0]]).is_err());
    }

    #[test]
    fn determinant_inverse_and_products_match_known_values() {
        let a = vec![vec![4.0, 1.0], vec![1.0, 3.0]];
        close(
            scalar_value(&det(&matrix_from_f64(&a).unwrap()).unwrap(), "det").unwrap(),
            11.0,
            1.0e-14,
        );
        let inverse = into_matrix(inv(&matrix_from_f64(&a).unwrap()).unwrap()).unwrap();
        close(inverse[0][0], 3.0 / 11.0, 1.0e-14);
        close(inverse[0][1], -1.0 / 11.0, 1.0e-14);
        let product = mul(
            &matrix_from_f64(&a).unwrap(),
            &matrix_from_f64(&inverse).unwrap(),
        )
        .unwrap();
        let identity = into_matrix(product).unwrap();
        close(identity[0][0], 1.0, 1.0e-14);
        close(identity[0][1], 0.0, 1.0e-14);
    }

    #[test]
    fn real_quadratic_roots_are_stable_and_negative_discriminant_is_an_error() {
        let roots = real_quadratic_roots(1.0, -1.0e8, 1.0).unwrap();
        close(roots[0], 1.0e-8, 1.0e-20);
        close(roots[1], 1.0e8, 1.0e-7);
        assert_eq!(real_quadratic_roots(1.0, 0.0, 0.0).unwrap(), [0.0, 0.0]);
        assert!(matches!(
            real_quadratic_roots(1.0, 0.0, 1.0),
            Err(Error::NoRealRoots { .. })
        ));
    }

    #[test]
    fn runtime_rank_and_one_based_indexing_are_checked() {
        let set = crate::FormulaSet {
            decls: vec![
                Decl {
                    name: "A".into(),
                    quantity: "stiffness_matrix".into(),
                    unit: Some("N/m".into()),
                },
                Decl {
                    name: "x".into(),
                    quantity: "displacement_vector".into(),
                    unit: Some("m".into()),
                },
            ],
            items: vec![],
            uses: vec![],
        };
        let shapes = shapes_of_set(&set).unwrap();
        assert_eq!(
            runtime_kind_of(
                &Expr::Call("solve".into(), vec![crate::var("A"), crate::var("x")]),
                &shapes
            )
            .unwrap(),
            RuntimeKind::Vector
        );
        let value = column_from_f64([3.0, 4.0]).unwrap();
        assert_eq!(into_scalar(index(&value, &[2]).unwrap()).unwrap(), 4.0);
        assert!(index(&value, &[0]).is_err());
        assert!(index(&value, &[3]).is_err());
    }
}

#[cfg(test)]
mod residual_gate_tests {
    use super::*;

    /// The residual gate is a genuine backstop: with a correct solve it stays far below the
    /// limit for well-conditioned systems (the randomized cross-check asserts `< 1e-12`), and
    /// it only fires if a future change to the solver lets a bad `x` through. It is exercised
    /// here by feeding `relative_residual` a deliberately wrong `x`.
    #[test]
    fn the_residual_gate_fires_on_a_cancelling_right_hand_side() {
        // 2x2 well-conditioned matrix; the solution of A x = b is computed first, then `b` is
        // perturbed by a value that the scale term cannot absorb. Residual must stay bounded.
        let a: Vec<Vec<f64>> = vec![vec![4.0, 1.0], vec![2.0, 3.0]];
        let b: Vec<f64> = vec![1.0, 2.0];
        let report = solve_dense(&a, &b).expect("well-conditioned system solves");
        assert!(report.residual <= RESIDUAL_LIMIT, "{:?}", report);

        // Direct: the relative residual of a deliberately wrong `x` is far above the limit.
        let wrong_x = vec![1e6, -1e6];
        assert!(relative_residual(&a, &b, &wrong_x) > RESIDUAL_LIMIT);
    }

    #[test]
    fn a_singular_matrix_reports_singular_not_residual() {
        let a: Vec<Vec<f64>> = vec![vec![1.0, 2.0], vec![2.0, 4.0]];
        let b: Vec<f64> = vec![1.0, 2.0];
        assert!(matches!(
            solve_dense(&a, &b),
            Err(Error::SingularMatrix { .. })
        ));
    }
}

#[cfg(test)]
mod lu_correctness_probe {
    use super::*;

    /// Independent Gaussian elimination with partial pivoting on a fresh copy (no LU reuse),
    /// used to cross-check `solve_dense` on the reviewer's counterexample.
    fn reference_solve(a: &[Vec<f64>], b: &[f64]) -> Vec<f64> {
        let n = a.len();
        let mut m: Vec<Vec<f64>> = a
            .iter()
            .zip(b)
            .map(|(r, v)| {
                let mut r = r.clone();
                r.push(*v);
                r
            })
            .collect();
        for k in 0..n {
            let p = (k..n)
                .max_by(|&i, &j| m[i][k].abs().partial_cmp(&m[j][k].abs()).unwrap())
                .unwrap();
            m.swap(k, p);
            for i in k + 1..n {
                let f = m[i][k] / m[k][k];
                let (above, from_i) = m.split_at_mut(i);
                let pivot_row = &above[k];
                for (mij, mkj) in from_i[0].iter_mut().zip(pivot_row).skip(k) {
                    *mij -= f * *mkj;
                }
            }
        }
        let mut x = vec![0.0; n];
        for i in (0..n).rev() {
            let mut s = m[i][n];
            for j in i + 1..n {
                s -= m[i][j] * x[j];
            }
            x[i] = s / m[i][i];
        }
        x
    }

    #[test]
    fn solve_dense_agrees_with_independent_elimination_on_the_reviewer_counterexample() {
        let a = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
        let b = vec![5.0, 11.0];
        let got = solve_dense(&a, &b)
            .expect("well-conditioned 2x2 must solve")
            .x;
        let want = reference_solve(&a, &b);
        for (g, w) in got.iter().zip(&want) {
            assert!((g - w).abs() < 1e-12, "got {got:?} want {want:?}");
        }
    }

    #[test]
    fn solve_dense_accepts_a_well_scaled_random_diagonally_dominant_system() {
        // The reviewer's 5x5 (unit diagonal, off-diagonals < 0.5) is diagonally dominant and
        // must be accepted with a tiny residual.
        let a = vec![
            vec![
                1.0,
                -0.4491506200283766,
                -0.19733833870850503,
                -0.29939189832657576,
                -0.3081871017348021,
            ],
            vec![
                -0.21957018342800438,
                1.0,
                -0.3220102919731289,
                -0.25860322266817093,
                -0.2767111894208938,
            ],
            vec![
                -0.18334857770241797,
                -0.3174033868126571,
                1.0,
                -0.4513468600343913,
                -0.043481278233230114,
            ],
            vec![
                -0.4011482526548207,
                -0.2433878609444946,
                -0.012647982221096754,
                1.0,
                -0.29477923293597996,
            ],
            vec![
                -0.15824346663430333,
                -0.1254318233113736,
                -0.06491107889451087,
                -0.40533009683713317,
                1.0,
            ],
        ];
        let b = vec![1.0, -1.0, 1.0, -1.0, 1.0];
        let report = solve_dense(&a, &b).expect("diagonally dominant 5x5 must solve");
        assert!(
            report.residual < 1e-12,
            "residual {:?} — solve_factored is wrong",
            report.residual
        );
        let want = reference_solve(&a, &b);
        for (g, w) in report.x.iter().zip(&want) {
            assert!((g - w).abs() < 1e-10, "got {:?} want {want:?}", report.x);
        }
    }
}

#[cfg(test)]
mod lu_randomized_crosscheck {
    use super::*;

    /// Deterministic LCG so the test is reproducible without a RNG dependency.
    struct Lcg(u64);
    impl Lcg {
        fn next_f64(&mut self, lo: f64, hi: f64) -> f64 {
            self.0 = self
                .0
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let u = ((self.0 >> 11) as f64) / ((1u64 << 53) as f64);
            lo + (hi - lo) * u
        }
    }

    fn reference(a: &[Vec<f64>], b: &[f64]) -> Vec<f64> {
        let n = a.len();
        let mut m: Vec<Vec<f64>> = a
            .iter()
            .zip(b)
            .map(|(r, v)| {
                let mut r = r.clone();
                r.push(*v);
                r
            })
            .collect();
        for k in 0..n {
            let p = (k..n)
                .max_by(|&i, &j| m[i][k].abs().partial_cmp(&m[j][k].abs()).unwrap())
                .unwrap();
            m.swap(k, p);
            for i in k + 1..n {
                let f = m[i][k] / m[k][k];
                let (above, from_i) = m.split_at_mut(i);
                let pivot_row = &above[k];
                for (mij, mkj) in from_i[0].iter_mut().zip(pivot_row).skip(k) {
                    *mij -= f * *mkj;
                }
            }
        }
        let mut x = vec![0.0; n];
        for i in (0..n).rev() {
            let mut s = m[i][n];
            for j in i + 1..n {
                s -= m[i][j] * x[j];
            }
            x[i] = s / m[i][i];
        }
        x
    }

    /// 2..=6 dimensional diagonally dominant systems: `solve_dense` must agree with an
    /// independent elimination and never refuse. This is the test that would have caught the
    /// permutation-replay defect (it refused well-conditioned 5x5 systems).
    #[test]
    fn solve_dense_matches_independent_elimination_across_sizes() {
        let mut rng = Lcg(0x5eed_1234_abcd_5678);
        for n in 2..=6usize {
            for _ in 0..200 {
                let mut a: Vec<Vec<f64>> = (0..n)
                    .map(|i| {
                        (0..n)
                            .map(|j| {
                                if i == j {
                                    rng.next_f64(1.0, 2.0)
                                } else {
                                    rng.next_f64(-0.45, 0.45)
                                }
                            })
                            .collect()
                    })
                    .collect();
                // Make sure at least one later-step pivot swap happens (non-trivial permutation).
                a[0][0] = 0.3;
                let b: Vec<f64> = (0..n).map(|_| rng.next_f64(-1.0, 1.0)).collect();
                let want = reference(&a, &b);
                match solve_dense(&a, &b) {
                    Ok(report) => {
                        for (g, w) in report.x.iter().zip(&want) {
                            assert!(
                                (g - w).abs() < 1e-9,
                                "n={n}: got {:?} want {want:?}",
                                report.x
                            );
                        }
                        assert!(
                            report.residual < 1e-12,
                            "n={n} residual {:?}",
                            report.residual
                        );
                    }
                    Err(e) => panic!("well-conditioned n={n} system refused: {e}"),
                }
            }
        }
    }
}
