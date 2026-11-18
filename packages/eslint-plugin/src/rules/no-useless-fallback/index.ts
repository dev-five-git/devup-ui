import {
  AST_NODE_TYPES,
  ESLintUtils,
  type TSESTree,
} from '@typescript-eslint/utils'

import { ImportStorage } from '../../utils/import-storage'
import { propertyKeyName } from '../../utils/property-key-name'
import { fallbackRulesLibrary } from '../../utils/responsive-position'
import { styleValueSite } from '../../utils/style-position'

const createRule = ESLintUtils.RuleCreator(
  (name) =>
    `https://github.com/dev-five-git/devup-ui/tree/main/packages/eslint-plugin/src/rules/${name}`,
)

/** Libraries whose rules declare each value of a property's array in turn, so a browser keeps the last one it understands. styled-components joins the values with commas into one declaration instead */
const FALLBACK_LIBRARIES = new Set([
  '@emotion/react',
  '@emotion/styled',
  '@emotion/css',
  '@vanilla-extract/css',
])

/** Devup UI shorthands, which read an array as breakpoints in every library's rules, as `GLOBAL_STYLE_PROPERTY` and `GLOBAL_ENUM_STYLE_PROPERTY` in `libs/css/src/constant.rs` list them, and keys holding no CSS value */
const DEVUP_KEYS = new Set([
  ...['bg', 'bgAttachment', 'bgClip', 'bgColor', 'bgImage', 'bgImg'],
  ...['bgOrigin', 'bgPosition', 'bgPositionX', 'bgPositionY', 'bgPos'],
  ...['bgPosX', 'bgPosY', 'bgRepeat', 'bgSize', 'bgBlendMode'],
  ...['backgroundImg', 'animationDir', 'flexDir', 'pos'],
  ...['m', 'mt', 'mr', 'mb', 'ml', 'mx', 'my'],
  ...['p', 'pt', 'pr', 'pb', 'pl', 'px', 'py'],
  ...['w', 'h', 'minW', 'minH', 'maxW', 'maxH', 'boxSize'],
  ...['borderBottomRadius', 'borderTopRadius', 'borderLeftRadius'],
  ...['borderRightRadius', 'objectPos', 'offsetPos', 'maskPos', 'maskImg'],
  ...['positioning', 'typography', 'styleOrder', 'selectors'],
])

/**
 * Syntax some browsers in use do not support yet, which an earlier value of
 * the array stands in for. Kept small on purpose: an entry matching none of
 * these is read as a plain value every browser supports.
 * - `var()` and `env()`, whose value is only known in the browser
 * - vendor prefixes: `-webkit-`, `-moz-`, `-ms-`, `-o-`
 * - small, large and dynamic viewport units (`dvh`, `svw`, `lvmin`...),
 *   container query units (`cqw`, `cqmin`...) and line height units (`lh`, `rlh`)
 * - CSS Color 4 and 5 functions: `oklch()`, `oklab()`, `lab()`, `lch()`,
 *   `hwb()`, `color()`, `color-mix()`, `light-dark()`, and `rgb()`/`hsl()`
 *   written without commas (`rgb(255 0 0 / 50%)`)
 * - comparison functions: `clamp()`, `min()`, `max()`
 */
const NEWER_SYNTAX: readonly RegExp[] = [
  /\b(?:var|env)\(/,
  /(?:^|[^\w-])-(?:webkit|moz|ms|o)-/,
  /\d(?:[sld]v(?:h|w|i|b|min|max)|cq(?:w|h|i|b|min|max)|r?lh)\b/,
  /\b(?:oklch|oklab|lab|lch|hwb|color|color-mix|light-dark)\(/,
  /\b(?:rgba?|hsla?)\([^,)]*\)/,
  /\b(?:clamp|min|max)\(/,
]

/** The text of `node` if it is a string written without substitutions */
function literalText(
  node: TSESTree.ArrayExpression['elements'][number],
): string | undefined {
  if (node?.type === AST_NODE_TYPES.Literal && typeof node.value === 'string')
    return node.value
  if (node?.type === AST_NODE_TYPES.TemplateLiteral && !node.expressions.length)
    return node.quasis[0].value.cooked ?? undefined
}

/** Whether every browser applies `value`, so nothing after it is needed as its fallback */
function isPlainValue(value: string | undefined): boolean {
  const text = value?.trim().toLowerCase()
  return !!text && !NEWER_SYNTAX.some((syntax) => syntax.test(text))
}

/** Whether `key` names a CSS property, whose array the library reads as it does rather than as breakpoints */
function isLibraryProperty(key: string | undefined): boolean {
  return key !== undefined && !DEVUP_KEYS.has(key) && !/^[_:&@]/.test(key)
}

export const noUselessFallback = createRule({
  name: 'no-useless-fallback',
  defaultOptions: [],
  meta: {
    schema: [],
    messages: {
      uselessFallback:
        "In {{library}} rules this array is a fallback list where only the last value ({{last}}) applies: every value in it is one all browsers support. If responsive values were intended, use the responsive arrays of Devup UI's `css()` or `<Box>`.",
    },
    type: 'suggestion',
    docs: {
      description:
        'No fallback arrays in Emotion and vanilla-extract rules whose earlier values never apply.',
    },
  },
  create(context) {
    const importStorage = new ImportStorage(context)
    return {
      ImportDeclaration(node) {
        importStorage.addImportByDeclaration(node)
      },
      ArrayExpression(node) {
        const property = node.parent
        if (
          property.type !== AST_NODE_TYPES.Property ||
          property.value !== node ||
          !isLibraryProperty(propertyKeyName(property)) ||
          node.elements.length < 2
        )
          return
        const values = node.elements.map(literalText)
        if (!values.every(isPlainValue)) return
        const site = styleValueSite(node, importStorage)
        const library = site && fallbackRulesLibrary(site.root, importStorage)
        if (!library || !FALLBACK_LIBRARIES.has(library)) return
        context.report({
          node,
          messageId: 'uselessFallback',
          data: { library, last: JSON.stringify(values[values.length - 1]) },
        })
      },
    }
  },
})
