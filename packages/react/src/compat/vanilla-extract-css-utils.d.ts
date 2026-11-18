declare module '@vanilla-extract/css-utils' {
  /**
   * The build compiles `calc` to the string it gives: in a `.css.ts`
   * stylesheet it is evaluated, elsewhere its operands must be literals or
   * module-level constants holding one.
   */
  type Operand = string | number | CalcChain

  /** What `calc(x)` builds, a CSS value once `toString()` is called */
  interface CalcChain {
    add: (...operands: Operand[]) => CalcChain
    subtract: (...operands: Operand[]) => CalcChain
    multiply: (...operands: Operand[]) => CalcChain
    divide: (...operands: Operand[]) => CalcChain
    negate: () => CalcChain
    toString: () => string
  }

  interface Calc {
    (x: Operand): CalcChain
    add: (...operands: Operand[]) => string
    subtract: (...operands: Operand[]) => string
    multiply: (...operands: Operand[]) => string
    divide: (...operands: Operand[]) => string
    negate: (x: Operand) => string
  }

  export const calc: Calc
}
