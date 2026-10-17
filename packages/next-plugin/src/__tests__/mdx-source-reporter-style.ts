export type ReporterNode = {
  readonly type: string
  readonly value?: string
  readonly children?: readonly ReporterNode[]
  readonly data?: { readonly estree: object }
}

export type ReporterTree = { readonly children: ReporterNode[] }

export function appendReportedStyle(tree: ReporterTree, color: unknown): void {
  if (
    !tree.children.some(
      (child) => child.type === 'mdxjsEsm' && child.value?.includes('css'),
    )
  )
    tree.children.unshift({
      type: 'mdxjsEsm',
      value: '',
      data: {
        estree: {
          type: 'Program',
          sourceType: 'module',
          body: [
            {
              type: 'ImportDeclaration',
              source: { type: 'Literal', value: '@devup-ui/react' },
              specifiers: [
                {
                  type: 'ImportSpecifier',
                  local: { type: 'Identifier', name: 'css' },
                  imported: { type: 'Identifier', name: 'css' },
                },
              ],
            },
          ],
        },
      },
    })
  tree.children.push({
    type: 'mdxjsEsm',
    value: '',
    data: {
      estree: {
        type: 'Program',
        sourceType: 'module',
        body: [
          {
            type: 'ExportNamedDeclaration',
            specifiers: [],
            declaration: {
              type: 'VariableDeclaration',
              kind: 'const',
              declarations: [
                {
                  type: 'VariableDeclarator',
                  id: { type: 'Identifier', name: 'injectedStyle' },
                  init: {
                    type: 'CallExpression',
                    callee: { type: 'Identifier', name: 'css' },
                    arguments: [
                      {
                        type: 'ObjectExpression',
                        properties: [
                          {
                            type: 'Property',
                            kind: 'init',
                            computed: false,
                            method: false,
                            shorthand: false,
                            key: { type: 'Identifier', name: 'color' },
                            value: { type: 'Literal', value: color },
                          },
                        ],
                      },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  })
}
