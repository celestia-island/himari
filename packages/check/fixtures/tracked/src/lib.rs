//! 尾巴闭环验证：formula_file!（登记清单）+ buildhelper::emit()（rerun-if-changed）。
//! `formula_file!` 必须显式带方言（「宏即方言」：文件内容无法从宏名推断，本宏不猜）。

/// `y = k*x + 1`（源：formulas/linear.formula）
pub fn linear(k: f64, x: f64) -> f64 {
    himari_macros::formula_file!("formulas/linear.formula", matlab)
}

#[cfg(test)]
mod tests {
    #[test]
    fn tracked_file_lane_value() {
        assert_eq!(super::linear(2.0, 3.0), 7.0);
    }
}
