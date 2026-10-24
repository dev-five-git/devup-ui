import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'

import { it } from 'bun:test'

import { parseReceipt } from './reservation-native-receipt.cjs'

const helper = resolve(import.meta.dir, 'reservation-native.cjs')
const owner = resolve(
  import.meta.dir,
  '../../../next-plugin/src/__tests__/native-webpack-owned.cjs',
)

it('reserves native Vite build and serve slots before opposite physical arrivals', () => {
  const root = mkdtempSync(join(tmpdir(), 'devup-vite-slots-'))
  /** @param {string} path @param {string} text */
  const put = (path, text) => {
    mkdirSync(dirname(join(root, path)), { recursive: true })
    writeFileSync(join(root, path), text)
  }
  put('index.html', '<script type="module" src="/src/entry.js"></script>')
  put('src/entry.js', 'import "./A.js"; import "./B.js"')
  put(
    'src/A.js',
    'import {css} from "@devup-ui/react"; console.log(css({color:"red",padding:"13px"}))',
  )
  put(
    'src/B.js',
    'import {css} from "@devup-ui/react"; console.log(css({color:"blue",margin:"17px"}))',
  )
  put('devup.json', '{}')
  try {
    for (const mode of ['build', 'serve']) {
      for (const single of ['single', 'per-file']) {
        const runs = []
        for (const order of ['AB', 'BA']) {
          const receipt = join(root, `${mode}-${single}-${order}.json`)
          const result = spawnSync(
            'node',
            [owner, '120000', helper, root, order, mode, single, receipt],
            { encoding: 'utf8', timeout: 150000 },
          )
          assert.equal(result.status, 0, result.stdout + result.stderr)
          const raw = readFileSync(receipt, 'utf8')
          if (process.env.VITE_SLOT_RECEIPTS) {
            mkdirSync(process.env.VITE_SLOT_RECEIPTS, { recursive: true })
            writeFileSync(
              join(
                process.env.VITE_SLOT_RECEIPTS,
                `${mode}-${single}-${order}.json`,
              ),
              raw,
            )
          }
          runs.push(parseReceipt(raw))
        }
        const [ab, ba] = runs
        assert(ab && ba)
        // Given identical physical inputs, When genuine native arrivals reverse,
        // Then engine getter outputs, counters and final build bytes agree.
        assert.deepEqual(
          ab.extractions.map((row) => [row.id, row.code, row.map]).sort(),
          ba.extractions.map((row) => [row.id, row.code, row.map]).sort(),
        )
        assert.equal(ab.classMap, ba.classMap)
        assert.equal(ab.fileMap, ba.fileMap)
        assert.equal(ab.count, 0)
        assert.equal(ba.count, 0)
        assert.deepEqual(ab.assets, ba.assets)
      }
    }
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
}, 600000)

it('reserves decoded static HTML script inputs outside conventional source roots', () => {
  const root = mkdtempSync(join(tmpdir(), 'devup-vite-html-slots-'))
  mkdirSync(join(root, 'outside'))
  writeFileSync(
    join(root, 'index.html'),
    '<script type="module" src="/outside/&copy;A.js"></script><script type="module" src="/outside/&copy;B.js"></script>',
  )
  writeFileSync(
    join(root, 'outside/©A.js'),
    'import {css} from "@devup-ui/react"; console.log(css({color:"red",padding:"13px"}))',
  )
  writeFileSync(
    join(root, 'outside/©B.js'),
    'import {css} from "@devup-ui/react"; console.log(css({color:"blue",margin:"17px"}))',
  )
  writeFileSync(join(root, 'devup.json'), '{}')
  try {
    const runs = []
    for (const order of ['AB', 'BA']) {
      const receipt = join(root, `html-${order}.json`)
      const result = spawnSync(
        'node',
        [owner, '120000', helper, root, order, 'build', 'single', receipt],
        { encoding: 'utf8', timeout: 150000 },
      )
      assert.equal(result.status, 0, result.stdout + result.stderr)
      const raw = readFileSync(receipt, 'utf8')
      if (process.env.VITE_SLOT_RECEIPTS) {
        mkdirSync(process.env.VITE_SLOT_RECEIPTS, { recursive: true })
        writeFileSync(
          join(process.env.VITE_SLOT_RECEIPTS, `html-${order}.json`),
          raw,
        )
        writeFileSync(
          join(process.env.VITE_SLOT_RECEIPTS, `html-${order}-lifecycle.log`),
          result.stdout + result.stderr,
        )
      }
      runs.push(parseReceipt(raw))
    }
    const [ab, ba] = runs
    assert(ab && ba)
    assert.equal(
      ab.extractions.filter((row) => /©[AB]\.js$/.test(row.id)).length,
      2,
    )
    assert.equal(
      ba.extractions.filter((row) => /©[AB]\.js$/.test(row.id)).length,
      2,
    )
    assert.equal(ab.count, 0)
    assert.equal(ba.count, 0)
    assert.equal(ab.classMap, ba.classMap)
    assert.deepEqual(ab.assets, ba.assets)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
}, 300000)
