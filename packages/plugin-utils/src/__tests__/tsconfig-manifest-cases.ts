export const manifestCases: readonly [
  string,
  unknown,
  string | undefined,
  string?,
][] = [
  [
    'versioned types matches',
    { exports: { 'types@>=6': './blue.json', default: './red.json' } },
    'blue.json',
  ],
  [
    'versioned types misses',
    { exports: { 'types@<6': './blue.json', default: './red.json' } },
    'red.json',
  ],
  [
    'invalid range misses',
    { exports: { 'types@bad': './blue.json', default: './red.json' } },
    'red.json',
  ],
  [
    'typesVersions root key is tsconfig',
    { typesVersions: { '*': { tsconfig: ['blue.json'] } } },
    'blue.json',
  ],
  [
    'typesVersions uses field path',
    {
      tsconfig: 'red.json',
      typesVersions: { '>=6': { 'red.json': ['blue.json'] } },
    },
    'blue.json',
  ],
  [
    'typesVersions deep maps',
    { typesVersions: { '^6.0.0': { 'config/*': ['*.json'] } } },
    'blue.json',
    'preset/config/blue',
  ],
  [
    'typesVersions first matching range',
    {
      typesVersions: {
        '*': { tsconfig: ['blue.json'] },
        '>=6': { tsconfig: ['red.json'] },
      },
    },
    'blue.json',
  ],
  [
    'exports overrides versions',
    {
      exports: './red.json',
      typesVersions: { '*': { tsconfig: ['blue.json'] } },
    },
    'red.json',
  ],
  ['malformed manifest defaults', '{', 'tsconfig.json'],
  [
    'JSONC manifest selects',
    '{/*comment*/"tsconfig":"blue.json",}',
    'blue.json',
  ],
  ['BOM manifest selects', '\uFEFF{"tsconfig":"blue.json"}', 'blue.json'],
  ['unquoted manifest defaults', '{tsconfig:"blue.json"}', 'tsconfig.json'],
  [
    'single quoted manifest defaults',
    "{'tsconfig':'blue.json'}",
    'tsconfig.json',
  ],
  [
    'hex manifest parse policy',
    '{"tsconfig":"blue.json","number":0x1}',
    'blue.json',
  ],
  [
    'hex overflow metadata retains tsconfig selection',
    `{"tsconfig":"blue.json","other":0x${'F'.repeat(400)}}`,
    'blue.json',
  ],
  [
    'separator overflow metadata retains tsconfig selection',
    '{"tsconfig":"blue.json","other":1_0e999}',
    'blue.json',
  ],
  [
    'hex overflow exports still encapsulates the package',
    `{"exports":0x${'F'.repeat(400)}}`,
    undefined,
  ],
  [
    'negative hex overflow metadata retains tsconfig selection',
    `{"tsconfig":"blue.json","other":-0x${'F'.repeat(400)}}`,
    'blue.json',
  ],
  [
    'negative separator overflow metadata retains tsconfig selection',
    '{"tsconfig":"blue.json","other":-1_0e999}',
    'blue.json',
  ],
  [
    'standard JSON decimal overflow retains tsconfig selection',
    '{"tsconfig":"blue.json","other":1e999}',
    'blue.json',
  ],
  [
    'binary manifest parse policy',
    '{"tsconfig":"blue.json","number":0b1}',
    'blue.json',
  ],
  [
    'octal manifest parse policy',
    '{"tsconfig":"blue.json","number":0o1}',
    'blue.json',
  ],
  [
    'numeric separator manifest parse policy',
    '{"tsconfig":"blue.json","number":1_000}',
    'blue.json',
  ],
  [
    'leading decimal manifest parse policy',
    '{"tsconfig":"blue.json","number":.5}',
    'blue.json',
  ],
  [
    'trailing decimal manifest parse policy',
    '{"tsconfig":"blue.json","number":1.}',
    'blue.json',
  ],
  [
    'invalid separator manifest defaults',
    '{"tsconfig":"blue.json","number":1__0}',
    'tsconfig.json',
  ],
  [
    'invalid hex manifest defaults',
    '{"tsconfig":"blue.json","number":0x}',
    'tsconfig.json',
  ],
  [
    'leading zero manifest parse policy',
    '{"tsconfig":"blue.json","number":01}',
    'tsconfig.json',
  ],
  [
    'undefined manifest defaults',
    '{"tsconfig":"blue.json","other":undefined}',
    'tsconfig.json',
  ],
  [
    'invalid requested subpath',
    { exports: { './*': './*.json' } },
    undefined,
    'preset/dir/../blue',
  ],
  [
    'invalid first target segment',
    { exports: './node_modules/blue.json' },
    undefined,
  ],
  [
    'typesVersions invalid value ignored',
    { typesVersions: { '*': 1 } },
    'tsconfig.json',
  ],
  [
    'typesVersions matched miss is terminal',
    { typesVersions: { '*': { tsconfig: ['missing.json'] } } },
    undefined,
  ],
  [
    'typesVersions invalid patterns ignored',
    { typesVersions: { '*': { '**': ['blue.json'] } } },
    'tsconfig.json',
  ],
  [
    'typesVersions targets miss then hit',
    { typesVersions: { '*': { tsconfig: ['missing.json', 'blue'] } } },
    'blue.json',
  ],
  [
    'version range misses use default',
    { typesVersions: { '<6': { tsconfig: ['blue.json'] } } },
    'tsconfig.json',
  ],
  [
    'version mapping long prefix wins',
    {
      typesVersions: {
        '*': { 'config/*': ['red.json'], 'config/long/*': ['blue.json'] },
      },
    },
    'blue.json',
    'preset/config/long/item',
  ],
  [
    'deep version matched miss stops default',
    { typesVersions: { '*': { 'blue.json': ['missing.json'] } } },
    undefined,
    'preset/blue.json',
  ],
]
