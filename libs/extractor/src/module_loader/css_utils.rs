//! `@vanilla-extract/css-utils`, which a stylesheet evaluates as the package
//! itself would: its `calc` builds the same strings, without a resolver

/// The package's specifier
pub(crate) const CSS_UTILS: &str = "@vanilla-extract/css-utils";

/// The object the package's exports are bound to while a stylesheet runs
pub(super) const CSS_UTILS_BINDING: &str = "__vanilla_extract_css_utils__";

/// What the error a `calc(x)` written as a CSS value throws says
pub(crate) const CHAIN_VALUE: &str = "of '@vanilla-extract/css-utils' builds an expression";

/// The package's `calc`, as `@vanilla-extract/css-utils` 0.1 writes it: the
/// operands of each operation are joined by the operator, every `calc` in
/// them dropped, so a nested operation is only parenthesized
pub(super) const CSS_UTILS_MODULE: &str = r#"const __vanilla_extract_css_utils__ = (() => {
const toExpression = (operator, ...operands) => operands.map((operand) => "".concat(operand)).join(" ".concat(operator, " ")).replace(/calc/g, "");
const add = (...operands) => "calc(".concat(toExpression("+", ...operands), ")");
const subtract = (...operands) => "calc(".concat(toExpression("-", ...operands), ")");
const multiply = (...operands) => "calc(".concat(toExpression("*", ...operands), ")");
const divide = (...operands) => "calc(".concat(toExpression("/", ...operands), ")");
const negate = (x) => multiply(x, -1);
const calc = Object.assign((x) => ({
  add: (...operands) => calc(add(x, ...operands)),
  subtract: (...operands) => calc(subtract(x, ...operands)),
  multiply: (...operands) => calc(multiply(x, ...operands)),
  divide: (...operands) => calc(divide(x, ...operands)),
  negate: () => calc(negate(x)),
  toString: () => x.toString(),
  toJSON: () => { throw new TypeError("`calc(x)` of '@vanilla-extract/css-utils' builds an expression, which is a CSS value only once `.toString()` is called or it is interpolated in a template"); },
}), { add, subtract, multiply, divide, negate });
return Object.freeze({ calc });
})();
"#;
