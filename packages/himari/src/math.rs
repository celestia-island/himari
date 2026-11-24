//! Small deterministic math wrappers shared by std and no_std builds.

/// Integer power by exponentiation by squaring, with no libm powf rounding policy.
pub(crate) fn powi(mut base: f64, exponent: i32) -> f64 {
    if exponent == 0 {
        return 1.0;
    }
    let negative = exponent < 0;
    let mut remaining = exponent.unsigned_abs();
    let mut result = 1.0;
    while remaining > 0 {
        if remaining & 1 == 1 {
            result *= base;
        }
        remaining >>= 1;
        if remaining > 0 {
            base *= base;
        }
    }
    if negative {
        1.0 / result
    } else {
        result
    }
}

pub(crate) fn powf(base: f64, exponent: f64) -> f64 {
    libm::pow(base, exponent)
}

pub(crate) fn fract(value: f64) -> f64 {
    libm::fmod(value, 1.0)
}

pub(crate) fn trunc(value: f64) -> f64 {
    libm::trunc(value)
}

pub(crate) fn sqrt(value: f64) -> f64 {
    libm::sqrt(value)
}

pub(crate) fn hypot(x: f64, y: f64) -> f64 {
    libm::hypot(x, y)
}

pub(crate) fn atan2(y: f64, x: f64) -> f64 {
    libm::atan2(y, x)
}

pub(crate) fn exp(value: f64) -> f64 {
    libm::exp(value)
}

pub(crate) fn ln(value: f64) -> f64 {
    libm::log(value)
}

pub(crate) fn sin(value: f64) -> f64 {
    libm::sin(value)
}

pub(crate) fn cos(value: f64) -> f64 {
    libm::cos(value)
}

pub(crate) fn sinh(value: f64) -> f64 {
    libm::sinh(value)
}

pub(crate) fn cosh(value: f64) -> f64 {
    libm::cosh(value)
}
