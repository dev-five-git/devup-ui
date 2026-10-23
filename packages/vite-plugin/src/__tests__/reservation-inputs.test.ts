import * as fs from 'node:fs'
import { join } from 'node:path'

import { expect, it, spyOn } from 'bun:test'

import { fixture } from '../../../plugin-utils/src/__tests__/reservation-fixture'
import {
  environmentInputs,
  physicalEnvironmentEntries,
} from '../reservation-inputs'

it.each([
  [{}, ['index.html']],
  [{ input: 'top.js' }, ['top.js']],
  [{ input: { a: 'A.js', b: 'B.js' } }, ['A.js', 'B.js']],
  [{ input: ['A.js', 'B.js'] }, ['A.js', 'B.js']],
  [
    { input: 'wrong.js', build: { rolldownOptions: { input: 'bundle.js' } } },
    ['bundle.js'],
  ],
  [{ build: { rollupOptions: { input: 'legacy.js' } } }, ['legacy.js']],
  [
    { build: { ssr: 'ssr.js', rolldownOptions: { input: 'wrong.js' } } },
    ['ssr.js'],
  ],
  [{ build: { lib: { entry: 'lib.js' }, ssr: 'wrong.js' } }, ['lib.js']],
  [
    {
      build: {
        lib: { entry: 'wrong.js' },
        rolldownOptions: { input: 'bundle.js' },
      },
    },
    ['bundle.js'],
  ],
  [{ build: { lib: { entry: ['A.js', 'B.js'] } } }, ['A.js', 'B.js']],
  [{ build: { lib: { entry: { a: 'A.js' } } } }, ['A.js']],
])('uses native effective input precedence for %j', (environment, expected) => {
  // Given / When / Then
  expect(environmentInputs(environment)).toEqual(expected)
})

it('applies native env define overrides before HTML parsing and keeps missing HTML watchable', async () => {
  // Given
  using project = fixture()
  project.write(
    'index.html',
    '<script type=module src="%BASE_URL%%NAME%/%N%/%BOOL%.js"></script>',
  )
  // When
  const result = await physicalEnvironmentEntries(
    {
      root: project.root,
      env: { BASE_URL: '/', NAME: 'wrong' },
      define: {
        'import.meta.env.NAME': '"right"',
        'import.meta.env.N': 3,
        'import.meta.env.BOOL': 'true',
        unrelated: false,
      },
    },
    { input: ['index.html', 'missing.html', 'static.js'] },
  )
  // Then
  expect(result.entries.map(({ request }) => request)).toEqual([
    join(project.root, 'right/3/true.js'),
    join(project.root, 'static.js'),
  ])
  expect(result.html).toEqual([
    join(project.root, 'index.html'),
    join(project.root, 'missing.html'),
  ])
})

it('preserves non-JSON define strings in HTML substitutions', async () => {
  // Given
  using project = fixture()
  project.write('index.html', '<script type=module src="%NAME%.js"></script>')
  // When
  const result = await physicalEnvironmentEntries(
    { root: project.root, define: { 'import.meta.env.NAME': 'raw' } },
    {},
  )
  // Then
  expect(result.entries.map(({ request }) => request)).toEqual([
    join(project.root, 'raw.js'),
  ])
})

it.each(['ENOENT', 'EISDIR', 'EACCES', 'EPERM', 'ENOTDIR'])(
  'keeps unreadable HTML watchable and discovers later entries after %s',
  async (code) => {
    // Given
    using project = fixture()
    project.write(
      'index.html',
      '<script type=module src="./entry.js"></script>',
    )
    using read = spyOn(fs, 'readFileSync').mockImplementationOnce(() => {
      throw Object.assign(new Error('HTML input is unavailable'), { code })
    })

    // When
    const result = await physicalEnvironmentEntries(
      { root: project.root },
      { input: ['unavailable.html', 'index.html', 'static.js'] },
    )

    // Then
    expect(read).toHaveBeenNthCalledWith(
      1,
      join(project.root, 'unavailable.html'),
      'utf8',
    )
    expect(result).toEqual({
      entries: [
        { request: join(project.root, 'entry.js'), context: project.root },
        { request: join(project.root, 'static.js'), context: project.root },
      ],
      html: [
        join(project.root, 'unavailable.html'),
        join(project.root, 'index.html'),
      ],
    })
  },
)

it('propagates unexpected physical HTML read failures rather than marking preparation ready', async () => {
  // Given
  using project = fixture()
  using read = spyOn(fs, 'readFileSync').mockImplementation(() => {
    throw new TypeError('unexpected read failure')
  })
  // When / Then
  await expect(
    physicalEnvironmentEntries({ root: project.root }, {}),
  ).rejects.toThrow(TypeError)
  expect(read).toHaveBeenCalledTimes(1)
})
