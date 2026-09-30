import { AST_NODE_TYPES, type TSESTree } from '@typescript-eslint/utils'

import type { ImportStorage } from './import-storage'

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

const STYLE_FUNCTIONS = new Set(['css', 'globalCss', 'keyframes'])

const STYLE_COMPONENTS = new Set([
  'Box',
  'Button',
  'Center',
  'Flex',
  'Grid',
  'Image',
  'Input',
  'Text',
  'VStack',
])

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

function isStyleComponent(
  name: TSESTree.JSXTagNameExpression,
  importStorage: ImportStorage,
): boolean {
  if (name.type === AST_NODE_TYPES.JSXIdentifier)
    return STYLE_COMPONENTS.has(importStorage.importedName(name.name) ?? '')
  return (
    name.type === AST_NODE_TYPES.JSXMemberExpression &&
    name.object.type === AST_NODE_TYPES.JSXIdentifier &&
    importStorage.isImportObject(name.object.name) &&
    STYLE_COMPONENTS.has(name.property.name)
  )
}

function isStyleFunction(
  callee: TSESTree.Expression,
  importStorage: ImportStorage,
): boolean {
  if (callee.type === AST_NODE_TYPES.Identifier)
    return STYLE_FUNCTIONS.has(importStorage.importedName(callee.name) ?? '')
  return (
    callee.type === AST_NODE_TYPES.MemberExpression &&
    !callee.computed &&
    callee.object.type === AST_NODE_TYPES.Identifier &&
    callee.property.type === AST_NODE_TYPES.Identifier &&
    importStorage.isImportObject(callee.object.name) &&
    STYLE_FUNCTIONS.has(callee.property.name)
  )
}

/** The Devup UI style component or style function closest above `node` */
export function styleRoot(
  node: TSESTree.Node,
  importStorage: ImportStorage,
): TSESTree.JSXOpeningElement | TSESTree.CallExpression | null {
  for (let current = node.parent; current; current = current.parent) {
    if (
      current.type === AST_NODE_TYPES.JSXOpeningElement &&
      isStyleComponent(current.name, importStorage)
    )
      return current
    if (
      current.type === AST_NODE_TYPES.CallExpression &&
      isStyleFunction(current.callee, importStorage)
    )
      return current
  }
  return null
}

/** Whether the build reads what `parent` holds in `child` as a style value: a style prop, a style object value, a responsive array, a branch of a condition or a spread */
function holdsStyle(parent: TSESTree.Node, child: TSESTree.Node): boolean {
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
    case AST_NODE_TYPES.JSXAttribute:
      return (
        parent.name.type === AST_NODE_TYPES.JSXIdentifier &&
        !isPassThroughProp(parent.name.name)
      )
    default:
      return false
  }
}

/** Whether the build reads `node` as a style value of `root`, every node between them holding it as a style */
export function isStylePosition(
  node: TSESTree.Node,
  root: TSESTree.Node,
): boolean {
  let child = node
  let parent = node.parent
  while (parent && parent !== root && holdsStyle(parent, child)) {
    child = parent
    parent = parent.parent
  }
  return parent === root
}

/** The Devup UI style component or function reading `node` as a style value, if one does */
export function styleValueRoot(
  node: TSESTree.Node,
  importStorage: ImportStorage,
): TSESTree.JSXOpeningElement | TSESTree.CallExpression | null {
  const root = styleRoot(node, importStorage)
  return root && isStylePosition(node, root) ? root : null
}
