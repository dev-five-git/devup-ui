import {
  AST_NODE_TYPES,
  type TSESLint,
  type TSESTree,
} from '@typescript-eslint/utils'

import {
  type ImportStorage,
  STYLE_COMPONENTS,
  STYLE_FUNCTIONS,
} from './import-storage'

/** The HTML, SVG and React attributes a Devup UI component passes through instead of reading as styles, as `is_special_property` in `libs/css/src/is_special_property.rs` lists them */
export const SPECIAL_PROPERTIES = new Set<string>([
  'dangerouslySetInnerHTML',
  'children',
  'key',
  'ref',
  'defaultChecked',
  'defaultValue',
  'suppressContentEditableWarning',
  'suppressHydrationWarning',
  'accessKey',
  'autoCapitalize',
  'autoFocus',
  'className',
  'contentEditable',
  'contextMenu',
  'dir',
  'draggable',
  'enterKeyHint',
  'hidden',
  'id',
  'lang',
  'nonce',
  'slot',
  'spellCheck',
  'style',
  'tabIndex',
  'title',
  'radioGroup',
  'role',
  'about',
  'datatype',
  'inlist',
  'prefix',
  'property',
  'rel',
  'resource',
  'rev',
  'typeof',
  'vocab',
  'autoCorrect',
  'autoSave',
  'itemProp',
  'itemScope',
  'itemType',
  'itemID',
  'itemRef',
  'results',
  'security',
  'unselectable',
  'popover',
  'popoverTargetAction',
  'popoverTarget',
  'inert',
  'inputMode',
  'is',
  'exportparts',
  'part',
  'accept',
  'acceptCharset',
  'action',
  'allowFullScreen',
  'allowTransparency',
  'alt',
  'async',
  'autoComplete',
  'autoPlay',
  'capture',
  'cellPadding',
  'cellSpacing',
  'charSet',
  'challenge',
  'checked',
  'cite',
  'classID',
  'cols',
  'colSpan',
  'controls',
  'coords',
  'crossOrigin',
  'data',
  'dateTime',
  'default',
  'defer',
  'disabled',
  'download',
  'encType',
  'form',
  'formAction',
  'formEncType',
  'formMethod',
  'formNoValidate',
  'formTarget',
  'frameBorder',
  'headers',
  'high',
  'href',
  'hrefLang',
  'htmlFor',
  'httpEquiv',
  'integrity',
  'keyParams',
  'keyType',
  'kind',
  'label',
  'list',
  'loop',
  'low',
  'manifest',
  'marginHeight',
  'marginWidth',
  'max',
  'maxLength',
  'media',
  'mediaGroup',
  'method',
  'min',
  'minLength',
  'multiple',
  'muted',
  'name',
  'noValidate',
  'open',
  'optimum',
  'pattern',
  'placeholder',
  'playsInline',
  'poster',
  'preload',
  'readOnly',
  'required',
  'reversed',
  'rows',
  'rowSpan',
  'sandbox',
  'scope',
  'scoped',
  'scrolling',
  'seamless',
  'selected',
  'shape',
  'size',
  'sizes',
  'span',
  'src',
  'srcDoc',
  'srcLang',
  'srcSet',
  'start',
  'step',
  'summary',
  'target',
  'type',
  'useMap',
  'value',
  'wmode',
  'wrap',
  'ping',
  'referrerPolicy',
  'allow',
  'loading',
  'decoding',
  'fetchPriority',
  'blocking',
  'imageSrcSet',
  'imageSizes',
  'precedence',
  'controlsList',
  'noModule',
  'align',
  'bgcolor',
  'frame',
  'rules',
  'dirName',
  'abbr',
  'valign',
  'disablePictureInPicture',
  'disableRemotePlayback',
  'accentHeight',
  'accumulate',
  'additive',
  'allowReorder',
  'alphabetic',
  'amplitude',
  'arabicForm',
  'ascent',
  'attributeName',
  'attributeType',
  'autoReverse',
  'azimuth',
  'baseFrequency',
  'baseProfile',
  'bbox',
  'begin',
  'bias',
  'by',
  'calcMode',
  'capHeight',
  'clipPathUnits',
  'colorProfile',
  'colorRendering',
  'contentScriptType',
  'contentStyleType',
  'decelerate',
  'descent',
  'diffuseConstant',
  'divisor',
  'dur',
  'dx',
  'dy',
  'edgeMode',
  'elevation',
  'enableBackground',
  'end',
  'exponent',
  'externalResourcesRequired',
  'filterRes',
  'filterUnits',
  'focusable',
  'format',
  'fr',
  'from',
  'fx',
  'fy',
  'g1',
  'g2',
  'glyphName',
  'glyphOrientationHorizontal',
  'glyphOrientationVertical',
  'glyphRef',
  'gradientTransform',
  'gradientUnits',
  'hanging',
  'horizAdvX',
  'horizOriginX',
  'ideographic',
  'in2',
  'in',
  'intercept',
  'k1',
  'k2',
  'k3',
  'k4',
  'k',
  'kernelMatrix',
  'kernelUnitLength',
  'kerning',
  'keyPoints',
  'keySplines',
  'keyTimes',
  'lengthAdjust',
  'limitingConeAngle',
  'local',
  'markerHeight',
  'markerUnits',
  'markerWidth',
  'maskContentUnits',
  'maskUnits',
  'mathematical',
  'mode',
  'numOctaves',
  'operator',
  'orient',
  'orientation',
  'origin',
  'overlinePosition',
  'overlineThickness',
  'panose1',
  'path',
  'pathLength',
  'patternContentUnits',
  'patternTransform',
  'patternUnits',
  'points',
  'pointsAtX',
  'pointsAtY',
  'pointsAtZ',
  'preserveAlpha',
  'preserveAspectRatio',
  'primitiveUnits',
  'radius',
  'refX',
  'refY',
  'renderingIntent',
  'repeatCount',
  'repeatDur',
  'requiredExtensions',
  'requiredFeatures',
  'restart',
  'result',
  'seed',
  'slope',
  'spacing',
  'specularConstant',
  'specularExponent',
  'speed',
  'spreadMethod',
  'startOffset',
  'stdDeviation',
  'stemh',
  'stemv',
  'stitchTiles',
  'strikethroughPosition',
  'strikethroughThickness',
  'string',
  'surfaceScale',
  'systemLanguage',
  'tableValues',
  'targetX',
  'targetY',
  'textLength',
  'to',
  'u1',
  'u2',
  'underlinePosition',
  'underlineThickness',
  'unicode',
  'unicodeRange',
  'unitsPerEm',
  'vAlphabetic',
  'values',
  'version',
  'vertAdvY',
  'vertOriginX',
  'vertOriginY',
  'vHanging',
  'vIdeographic',
  'viewBox',
  'viewTarget',
  'vMathematical',
  'widths',
  'x1',
  'x2',
  'xChannelSelector',
  'xHeight',
  'xlinkActuate',
  'xlinkArcrole',
  'xlinkHref',
  'xlinkRole',
  'xlinkShow',
  'xlinkTitle',
  'xlinkType',
  'xmlBase',
  'xmlLang',
  'xmlns',
  'xmlnsXlink',
  'xmlSpace',
  'y1',
  'y2',
  'yChannelSelector',
  'z',
  'zoomAndPan',
  'allowpopups',
  'autosize',
  'blinkfeatures',
  'disableblinkfeatures',
  'disableguestresize',
  'disablewebsecurity',
  'guestinstance',
  'httpreferrer',
  'nodeintegration',
  'partition',
  'plugins',
  'useragent',
  'webpreferences',
])

/** Props a Devup UI component reads itself rather than as styles */
const OWN_PROPS = new Set(['as', 'props', 'styleVars', 'styleOrder'])

/** Keys of style objects holding data rather than style values */
const DATA_KEYS = new Set(['imports', 'fontFaces', 'params'])

type Variable = TSESLint.Scope.Variable

/** Whether a Devup UI component passes the prop `name` through instead of reading it as a style */
export function isPassThroughProp(name: string): boolean {
  return (
    name.startsWith('on') ||
    name.startsWith('data-') ||
    name.startsWith('aria-') ||
    SPECIAL_PROPERTIES.has(name) ||
    OWN_PROPS.has(name)
  )
}

/** The Devup UI name `expression` reads: an import by name, or a member of the package imported whole */
function devupName(
  expression: TSESTree.Node,
  importStorage: ImportStorage,
): string | undefined {
  if (expression.type === AST_NODE_TYPES.Identifier)
    return importStorage.importedName(expression.name)
  if (
    expression.type === AST_NODE_TYPES.MemberExpression &&
    !expression.computed &&
    expression.object.type === AST_NODE_TYPES.Identifier &&
    expression.property.type === AST_NODE_TYPES.Identifier &&
    importStorage.isImportObject(expression.object.name)
  )
    return expression.property.name
  return undefined
}

/** The Devup UI component a JSX name reads, if it reads one */
export function componentName(
  name: TSESTree.JSXTagNameExpression,
  importStorage: ImportStorage,
): string | undefined {
  if (name.type === AST_NODE_TYPES.JSXIdentifier)
    return importStorage.importedName(name.name)
  return name.type === AST_NODE_TYPES.JSXMemberExpression &&
    name.object.type === AST_NODE_TYPES.JSXIdentifier &&
    importStorage.isImportObject(name.object.name)
    ? name.property.name
    : undefined
}

/** Whether `node` is a JSX element the build reads styles from: a style component, or Emotion's `Global` with its `styles` */
function isStyleElement(
  node: TSESTree.JSXOpeningElement,
  importStorage: ImportStorage,
): boolean {
  const name = componentName(node.name, importStorage) ?? ''
  return STYLE_COMPONENTS.has(name) || name === 'Global'
}

export function isStyledReference(
  node: TSESTree.Node,
  importStorage: ImportStorage,
): boolean {
  return devupName(node, importStorage) === 'styled'
}

/** Whether `node` still waits for rules: `styled.div`, `styled(tag)`, and what `.attrs()` and `.withConfig()` give */
export function isStyledFactory(
  node: TSESTree.Node,
  importStorage: ImportStorage,
): boolean {
  if (node.type === AST_NODE_TYPES.MemberExpression)
    return (
      !node.computed &&
      node.property.type === AST_NODE_TYPES.Identifier &&
      isStyledReference(node.object, importStorage)
    )
  if (node.type !== AST_NODE_TYPES.CallExpression) return false
  const callee = node.callee
  if (callee.type === AST_NODE_TYPES.MemberExpression)
    return (
      !callee.computed &&
      callee.property.type === AST_NODE_TYPES.Identifier &&
      (callee.property.name === 'attrs' ||
        callee.property.name === 'withConfig') &&
      isStyledFactory(callee.object, importStorage)
    )
  return node.arguments.length === 1 && isStyledReference(callee, importStorage)
}

/** The rules `styled(tag, rules)` takes in its second argument, which must be an object literal for the build to read it */
function styledRulesOf(
  call: TSESTree.CallExpression,
  importStorage: ImportStorage,
): TSESTree.CallExpressionArgument[] {
  const rules = call.arguments[1]
  return call.arguments.length === 2 &&
    isStyledReference(call.callee, importStorage) &&
    rules.type === AST_NODE_TYPES.ObjectExpression
    ? [rules]
    : []
}

/** The arguments of `call` the build reads as styles: those of `css`, `globalCss`, `keyframes` and `createGlobalStyle`, of a styled component given its tag first (`styled.div({ ... })`), and the rules of `styled(tag, { ... })`. Null if it reads none */
export function styleArguments(
  call: TSESTree.CallExpression,
  importStorage: ImportStorage,
): TSESTree.CallExpressionArgument[] | null {
  if (STYLE_FUNCTIONS.has(devupName(call.callee, importStorage) ?? ''))
    return call.arguments
  if (isStyledFactory(call.callee, importStorage)) return call.arguments
  const rules = styledRulesOf(call, importStorage)
  return rules.length > 0 ? rules : null
}

/** The Devup UI style component or style function closest above `node` */
export function styleRoot(
  node: TSESTree.Node,
  importStorage: ImportStorage,
): TSESTree.JSXOpeningElement | TSESTree.CallExpression | null {
  for (
    let current: TSESTree.Node | undefined = node.parent;
    current;
    current = current.parent
  ) {
    if (
      current.type === AST_NODE_TYPES.JSXOpeningElement &&
      isStyleElement(current, importStorage)
    )
      return current
    if (
      current.type === AST_NODE_TYPES.CallExpression &&
      styleArguments(current, importStorage)
    )
      return current
  }
  return null
}

/** Whether the build reads what `parent` holds in `child` as a style value: a style prop, a style object value, a responsive array, a branch of a condition or a spread */
function holdsStyle(
  parent: TSESTree.Node,
  child: TSESTree.Node,
  importStorage: ImportStorage,
): boolean {
  switch (parent.type) {
    case AST_NODE_TYPES.ObjectExpression:
    case AST_NODE_TYPES.ArrayExpression:
    case AST_NODE_TYPES.SpreadElement:
    case AST_NODE_TYPES.LogicalExpression:
    case AST_NODE_TYPES.JSXExpressionContainer:
    case AST_NODE_TYPES.JSXSpreadAttribute:
    case AST_NODE_TYPES.TSAsExpression:
    case AST_NODE_TYPES.TSSatisfiesExpression:
    case AST_NODE_TYPES.TSNonNullExpression:
      return true
    case AST_NODE_TYPES.Property:
      return (
        parent.value === child &&
        !(
          parent.key.type === AST_NODE_TYPES.Identifier &&
          !parent.computed &&
          DATA_KEYS.has(parent.key.name)
        )
      )
    case AST_NODE_TYPES.ConditionalExpression:
      return parent.test !== child
    case AST_NODE_TYPES.JSXAttribute: {
      if (parent.name.type !== AST_NODE_TYPES.JSXIdentifier) return false
      return componentName(parent.parent.name, importStorage) === 'Global'
        ? parent.name.name === 'styles'
        : !isPassThroughProp(parent.name.name)
    }
    default:
      return false
  }
}

/** Where a style value the build reads sits: below `root`, from `start` down. `start` is `root`, or the value of a module-level `const` every use of which the build reads as a style */
export interface StyleSite {
  root: TSESTree.JSXOpeningElement | TSESTree.CallExpression
  start: TSESTree.Node
}

/** The root the build reads `child` from when it is the direct style value of `root`: an attribute of the element, or an argument it reads */
function readsDirectly(
  child: TSESTree.Node,
  root: TSESTree.JSXOpeningElement | TSESTree.CallExpression,
  importStorage: ImportStorage,
): boolean {
  return (
    root.type !== AST_NODE_TYPES.CallExpression ||
    (styleArguments(root, importStorage) ?? []).includes(
      child as TSESTree.CallExpressionArgument,
    )
  )
}

/** Where the build reads `node` as a style value, every node between holding it as a style. Through a module-level `const` the build inlines, if it is not exported and nothing but styles reads it */
export function styleValueSite(
  node: TSESTree.Node,
  importStorage: ImportStorage,
  visiting: ReadonlySet<Variable> = new Set(),
): StyleSite | null {
  const root = styleRoot(node, importStorage)
  let child = node
  let parent = node.parent
  while (
    parent &&
    parent !== root &&
    holdsStyle(parent, child, importStorage)
  ) {
    child = parent
    parent = parent.parent
  }
  if (root && parent === root)
    return readsDirectly(child, root, importStorage)
      ? { root, start: root }
      : null
  return parent?.type === AST_NODE_TYPES.VariableDeclarator &&
    parent.init === child
    ? constSite(parent, child, importStorage, visiting)
    : null
}

function constSite(
  declarator: TSESTree.VariableDeclarator,
  init: TSESTree.Node,
  importStorage: ImportStorage,
  visiting: ReadonlySet<Variable>,
): StyleSite | null {
  const declaration = declarator.parent
  if (
    declarator.id.type !== AST_NODE_TYPES.Identifier ||
    declaration.kind !== 'const' ||
    declaration.parent.type !== AST_NODE_TYPES.Program
  )
    return null
  const [variable] = importStorage.declaredVariables(declarator)
  if (!variable || visiting.has(variable)) return null
  const reads = variable.references.filter((reference) => !reference.init)
  if (reads.length === 0) return null
  const inner = new Set(visiting).add(variable)
  let site: StyleSite | null = null
  for (const reference of reads) {
    const read = reference.isWrite()
      ? null
      : styleValueSite(reference.identifier, importStorage, inner)
    if (!read) return null
    site ??= { root: read.root, start: init }
  }
  return site
}

/** The Devup UI style component or function reading `node` as a style value, if one does */
export function styleValueRoot(
  node: TSESTree.Node,
  importStorage: ImportStorage,
): TSESTree.JSXOpeningElement | TSESTree.CallExpression | null {
  return styleValueSite(node, importStorage)?.root ?? null
}
