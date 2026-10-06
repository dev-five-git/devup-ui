import {
  cssUtilsLiteralOnly,
  noDuplicateValue,
  noTypographyTokenPrefix,
  noUselessResponsive,
  noUselessTailingNulls,
  preferMediaShorthand,
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
          'no-typography-token-prefix': noTypographyTokenPrefix,
          'prefer-media-shorthand': preferMediaShorthand,
        },
      },
    },
    rules: {
      '@devup-ui/no-useless-tailing-nulls': 'error',
      '@devup-ui/css-utils-literal-only': 'error',
      '@devup-ui/no-duplicate-value': 'error',
      '@devup-ui/no-useless-responsive': 'error',
      '@devup-ui/style-order-range': 'error',
      '@devup-ui/no-typography-token-prefix': 'error',
      '@devup-ui/prefer-media-shorthand': 'warn',
    },
  },
]
