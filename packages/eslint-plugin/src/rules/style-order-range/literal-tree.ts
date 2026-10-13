import { AST_NODE_TYPES, type TSESTree } from '@typescript-eslint/utils'

import { bounds, cssValue, directives, trivia } from './css-scanner'
import { finiteValues } from './literal-value'
import { type ScopeOf, staticValue, validOrder } from './static-value'
import type { Site } from './style-tree'
import { sourceText } from './text-origin'

export type TextDiagnostic = {
  readonly start: number
  readonly end: number
  readonly site: Site
  readonly messageId: 'unsupportedOrder' | 'globalOrder' | 'styleOrderRange'
}

export function createLiteralTree(
  scopeOf: ScopeOf,
  hooks: {
    readonly check: (node: TSESTree.Node, site: Site) => void
    readonly report: (diagnostic: TextDiagnostic) => void
    readonly walk: (node: TSESTree.Node, site: Site) => void
  },
) {
  return (node: TSESTree.Literal | TSESTree.TemplateLiteral, site: Site) => {
    if (
      site.text === false ||
      site.allowText === false ||
      (node.type === AST_NODE_TYPES.Literal && typeof node.value !== 'string')
    )
      return
    const source = sourceText(node)
    const found = directives(source, site.mode)
    for (const directive of found.directives) {
      const local = { ...site, mode: directive.mode }
      const holes = [...source.holes].filter(
        ([index]) =>
          found.visibleHoles.has(index) &&
          index >= directive.start &&
          index < directive.end,
      )
      const range = bounds(source.text.slice(directive.start, directive.end))
      const start = directive.start + range[0]
      const end = directive.start + range[1]
      const report = (
        messageId: TextDiagnostic['messageId'],
        expression?: TSESTree.Node,
      ) =>
        hooks.report({
          start:
            expression?.range[0] ??
            source.offsets[
              messageId === 'unsupportedOrder' ? directive.key : start
            ],
          end:
            expression?.range[1] ??
            source.offsets[
              messageId === 'unsupportedOrder' ? directive.keyEnd : end
            ],
          site: local,
          messageId,
        })
      if (
        directive.mode === 'keyframes' ||
        directive.mode === 'fontface' ||
        directive.mode === 'stylex'
      ) {
        report('unsupportedOrder')
        continue
      }
      if (
        holes.length === 1 &&
        trivia(source.text.slice(start, end)).trim() === '\ufffc'
      ) {
        const expression = holes[0][1]
        if (local.mode === 'class') {
          const values = finiteValues(expression, scopeOf, {
            callback: site.callbacks === true,
            seen: new Set(),
          })
          if (values) {
            const invalid = values.find((value) => !validOrder(value.value))
            if (invalid) report('styleOrderRange', invalid.node)
            continue
          }
        }
        hooks.check(expression, local)
        continue
      }
      if (holes.length === 0) {
        if (!validOrder(cssValue(source.text.slice(start, end))))
          report('styleOrderRange')
        else if (site.conditional) report('globalOrder')
        continue
      }
      let strings = ['']
      const quoted =
        source.text[start] === '"' || source.text[start] === "'"
          ? source.text[start]
          : ''
      let cursor = start
      let invalid: TSESTree.Node | undefined
      let finite = true
      for (const [index, expression] of holes) {
        const values = finiteValues(expression, scopeOf, {
          callback: site.callbacks === true,
          seen: new Set(),
          absence: false,
        })
        if (!values || strings.length * values.length > 256) {
          finite = false
          invalid = expression
          break
        }
        const head = trivia(
          source.text.slice(cursor, index),
          cursor === start ? '' : quoted,
          ' ',
        )
        strings = strings.flatMap((prefix) =>
          values.map((value) => prefix + head + String(value.value)),
        )
        cursor = index + 1
      }
      if (
        !finite ||
        strings.some((value) => {
          const token =
            value + trivia(source.text.slice(cursor, end), quoted, ' ')
          return !validOrder(quoted ? cssValue(token) : token)
        })
      )
        report('styleOrderRange', invalid)
      else if (
        local.mode === 'global' &&
        (site.conditional ||
          holes.some(([, expression]) => !staticValue(expression, scopeOf)))
      )
        report('globalOrder', holes[0][1])
    }
    for (const mixin of found.mixins) {
      hooks.walk(mixin.node, {
        ...site,
        mode: mixin.mode,
        returnsRules: site.callbacks,
      })
    }
  }
}
