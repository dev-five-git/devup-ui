import {
  cssUtilsLiteralOnly,
  noDuplicateValue,
  noUselessResponsive,
  noUselessTailingNulls,
  styleOrderRange,
} from '../rules'

export default [
  {
    plugins: {
      '@devup-ui': {
        rules: {
          'no-useless-tailing-nulls': noUselessTailingNulls,
          'css-utils-literal-only': cssUtilsLiteralOnly,
          'no-duplicate-value': noDuplicateValue,
          'no-useless-responsive': noUselessResponsive,
          'style-order-range': styleOrderRange,
        },
      },
    },
    rules: {
      '@devup-ui/no-useless-tailing-nulls': 'error',
      '@devup-ui/css-utils-literal-only': 'error',
      '@devup-ui/no-duplicate-value': 'error',
      '@devup-ui/no-useless-responsive': 'error',
      '@devup-ui/style-order-range': 'error',
    },
  },
]
