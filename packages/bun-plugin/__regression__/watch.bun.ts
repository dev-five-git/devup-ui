import {
  mkdirSync,
  mkdtempSync,
  realpathSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

import { expect, it } from 'bun:test'

it('reextracts an importer when only its build-time token changes under normal Bun watch caching', async () => {
  const scratch = join(
    tmpdir(),
    'opencode',
    'workers',
    'w20-plugins-core',
    'bun',
  )
  mkdirSync(scratch, { recursive: true })
  const root = realpathSync.native(mkdtempSync(join(scratch, 'watch-')))
  const plugin = resolve(import.meta.dir, '../src/register.ts').replaceAll(
    '\\',
    '/',
  )
  writeFileSync(join(root, 'bunfig.toml'), '')
  writeFileSync(join(root, 'tokens.ts'), "export const width = '751px'")
  writeFileSync(
    join(root, 'style.css.ts'),
    "import { style } from '@devup-ui/react'; import { width } from './tokens'; export const cls = style({ width })",
  )
  writeFileSync(
    join(root, 'check.ts'),
    `import { register } from ${JSON.stringify(plugin)};
await register({ debug: true });
const { cls } = await import('./style.css.ts');
console.log('WATCH_RESULT:' + JSON.stringify({ cls, css: await Bun.file('df/devup-ui/devup-ui.css').text() }));`,
  )
  const child = Bun.spawn([process.execPath, '--watch', 'check.ts'], {
    cwd: root,
    stdout: 'pipe',
    stderr: 'pipe',
  })
  const errors = new Response(child.stderr).text()
  const timer = setTimeout(() => child.kill(), 15000)
  const results: string[] = []
  try {
    let pending = ''
    for await (const chunk of child.stdout) {
      pending += new TextDecoder().decode(chunk)
      const lines = pending.split('\n')
      pending = lines.pop() ?? ''
      for (const line of lines) {
        if (!line.startsWith('WATCH_RESULT:')) continue
        results.push(line)
        if (results.length === 1)
          writeFileSync(join(root, 'tokens.ts'), "export const width = '757px'")
      }
      if (results.length >= 2) break
    }
    child.kill()
    expect(results, (await errors) + results.join('\n')).toHaveLength(2)
    expect(results[0]).toContain('width:751px')
    expect(results[1]).toContain('width:757px')
    expect(results[0]).toMatch(/"cls":"[^"]*751px/)
    expect(results[1]).toMatch(/"cls":"[^"]*757px/)
    expect(results[1]).not.toContain('width:751px')
    expect(results[1]).not.toBe(results[0])
  } finally {
    clearTimeout(timer)
    child.kill()
    await child.exited
    rmSync(root, { recursive: true, force: true })
  }
}, 20000)
