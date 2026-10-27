import { existsSync, readFileSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'

import { expect, it } from 'bun:test'

import { nativePublicationFixture } from '../../../../test-harness/native-publication-fixture'
import {
  beginPublication,
  recordRoot,
} from '../../../../test-harness/publication'

it.each(['css', 'mdx', 'snap'])(
  'rejects drift in actual native-read %s input bytes',
  async (extension) => {
    // Given source whose real native result depends on a package fixture input.
    const fixture = nativePublicationFixture()
    const input = join(fixture.root, `packages/value.${extension}`)
    writeFileSync(input, '/*42*/')
    writeFileSync(
      join(fixture.root, 'bun.lock'),
      '{"lockfileVersion":1,"workspaces":{}}',
    )
    writeFileSync(
      join(fixture.root, 'source.ts'),
      `import {readFileSync} from 'node:fs';export function value(){return Number(readFileSync(new URL('./packages/value.${extension}',import.meta.url),'utf8').match(/\\d+/)?.[0])}\n`,
    )
    try {
      beginPublication(fixture.root)
      const result = await fixture.lane('root')
      expect(result.code, result.stderr).toBe(0)
      console.info(
        JSON.stringify({
          event: 'native-input-drift',
          extension,
          raw: readFileSync(
            join(fixture.root, 'coverage/root/lcov.info'),
            'utf8',
          ),
        }),
      )
      // When consumed input changes after successful native measurement.
      writeFileSync(input, '/*99*/')
      // Then stale coverage cannot qualify the changed input.
      expect(() => recordRoot(fixture.root, ['source.ts'])).toThrow(
        'identity changed',
      )
      expect(existsSync(join(fixture.root, 'coverage/accepted.json'))).toBe(
        false,
      )
    } finally {
      fixture.dispose()
    }
  },
  120000,
)
