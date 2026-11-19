import { spawnSync } from 'node:child_process'
import {
  existsSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

import { afterEach, beforeEach, describe, expect, it } from 'bun:test'

import storedHistory from '../../../../benchmark/history.json'
import {
  addToHistory,
  applyRun,
  formatJson,
  HISTORY_COLUMNS,
  HISTORY_LIMIT,
  historyEntry,
  refreshPlan,
  round,
  sizeSummary,
  summarizeRow,
  trend,
} from '../../../../benchmark-record.js'
import storedResults from '../../../../benchmark-results.json'

const ROOT = resolve(import.meta.dir, '../../../..')
const SHA_A = 'a'.repeat(40)
const SHA_B = 'b'.repeat(40)

function measured(median: number, extra: Record<string, unknown> = {}) {
  return {
    samples: [median - 0.1, median, median + 0.2],
    median,
    min: median - 0.1,
    max: median + 0.2,
    bytes: 1000,
    cssBytes: 100,
    ...extra,
  }
}

function makeRun(commit: string, rows: Record<string, unknown>) {
  return {
    run: { id: '7', commit, date: '2026-11-15T00:00:00.000Z' },
    samples: 3,
    rows,
  }
}

const results = {
  run: {
    id: '1',
    url: 'https://github.com/dev-five-git/devup-ui/actions/runs/1',
    commit: 'aaaaaaaa',
    sha: SHA_A,
    date: '2026-11-01T00:00:00.000Z',
    runner: 'ubuntu-24.04',
    next: '16.0.0',
    devupUi: '1.0.0',
  },
  samples: 3,
  rows: [
    {
      table: 'webpack',
      target: 'tailwind',
      label: 'tailwindcss',
      version: '4.0.0',
    },
  ],
}

describe('benchmark rows', () => {
  it('rounds seconds to the millisecond', () => {
    expect(round(10.23456)).toBe(10.235)
  })

  it('summarizes a size measured on every sample', () => {
    expect(sizeSummary([3, 1, 2])).toEqual({
      samples: [3, 1, 2],
      median: 2,
      min: 1,
      max: 3,
    })
  })

  it('reports the median of every measurement and keeps the size samples', () => {
    const row = summarizeRow({
      seconds: [3, 1, 2],
      bytes: [10, 12, 11],
      cssBytes: [5, 5, 5],
      clientBytes: [40, 41, 39],
    })

    expect(row).toEqual({
      samples: [3, 1, 2],
      median: 2,
      min: 1,
      max: 3,
      bytes: 11,
      cssBytes: 5,
      clientBytes: 40,
      sizes: {
        bytes: { samples: [10, 12, 11], median: 11, min: 10, max: 12 },
        cssBytes: { samples: [5, 5, 5], median: 5, min: 5, max: 5 },
        clientBytes: { samples: [40, 41, 39], median: 40, min: 39, max: 41 },
      },
    })
  })
})

describe('benchmark results', () => {
  it('takes the run, its metadata and every measurement of the table rows', () => {
    const sizes = { bytes: sizeSummary([1000, 1000, 1000]) }
    const run = {
      run: {
        id: '9',
        commit: SHA_B,
        date: '2026-11-15T00:00:00.000Z',
        runner: 'ubuntu-26.04',
        next: '16.4.6',
        devupUi: '1.0.45',
        bytesDefinition: 'deployable',
      },
      samples: 3,
      rows: {
        tailwind: measured(10.23456, {
          clientBytes: 500,
          sizes,
          version: '4.3.5',
        }),
        unused: measured(1),
      },
    }

    expect(applyRun(results, run)).toEqual({
      run: {
        id: '9',
        url: 'https://github.com/dev-five-git/devup-ui/actions/runs/9',
        commit: 'bbbbbbbb',
        sha: SHA_B,
        date: '2026-11-15T00:00:00.000Z',
        runner: 'ubuntu-26.04',
        next: '16.4.6',
        devupUi: '1.0.45',
        bytesDefinition: 'deployable',
      },
      samples: 3,
      rows: [
        {
          table: 'webpack',
          target: 'tailwind',
          label: 'tailwindcss',
          version: '4.3.5',
          samples: [10.135, 10.235, 10.435],
          median: 10.235,
          min: 10.135,
          max: 10.435,
          bytes: 1000,
          cssBytes: 100,
          clientBytes: 500,
          sizes,
        },
      ],
    })
  })

  it('keeps what an older run did not record', () => {
    const run = {
      run: { id: null, commit: null },
      samples: 3,
      rows: { tailwind: measured(10) },
    }

    const next = applyRun(results, run)

    // Its bytes were counted before the definition was recorded
    expect(next.run).toEqual({ ...results.run, bytesDefinition: 'output-dir' })
    expect(next.rows[0]).toMatchObject({
      version: '4.0.0',
      clientBytes: null,
      sizes: null,
    })
  })

  it('rejects a run without a row of the tables', () => {
    expect(() => applyRun(results, makeRun(SHA_B, {}))).toThrow(
      'the run has no tailwind row for the tables',
    )
  })

  it('keeps the checked results in the shape the README and landing read', () => {
    expect(storedResults.run.sha).toStartWith(storedResults.run.commit)
    expect(storedResults.run.url).toEndWith(storedResults.run.id)
    for (const row of storedResults.rows) {
      expect(row.min).toBeLessThanOrEqual(row.median)
      expect(row.median).toBeLessThanOrEqual(row.max)
      expect(row.bytes).toBeGreaterThan(0)
    }
  })
})

describe('benchmark history', () => {
  const baseline = {
    samples: 3,
    rows: {
      'devup-ui': {
        control: 'tailwind',
        ratio: 0.9,
        timeTolerance: 0.1,
        bytes: 1000,
        cssBytes: 100,
      },
    },
  }

  it('records every row and the gate verdict of a run', () => {
    const run = makeRun(SHA_A, {
      tailwind: measured(10, { clientBytes: 50 }),
      'devup-ui': measured(9.12345),
    })

    expect(historyEntry(run, baseline, '2026-11-14T00:00:00.000Z')).toEqual({
      sha: SHA_A,
      run: '7',
      committedAt: '2026-11-14T00:00:00.000Z',
      measuredAt: '2026-11-15T00:00:00.000Z',
      devupUi: null,
      samples: 3,
      bytesDefinition: 'output-dir',
      gate: 'pass',
      regressions: [],
      rows: {
        tailwind: [10, 9.9, 10.2, 1000, 100, 50],
        'devup-ui': [9.123, 9.023, 9.323, 1000, 100, null],
      },
    })
  })

  it('records which bytes a run counted', () => {
    const run = {
      ...makeRun(SHA_A, {
        tailwind: measured(10),
        'devup-ui': measured(9),
      }),
      run: { id: '7', commit: SHA_A, bytesDefinition: 'deployable' },
    }

    expect(
      historyEntry(run, baseline, '2026-11-14T00:00:00.000Z'),
    ).toMatchObject({ bytesDefinition: 'deployable', gate: 'fail' })
    expect(
      historyEntry(
        run,
        { ...baseline, bytesDefinition: 'deployable' },
        '2026-11-14T00:00:00.000Z',
      ),
    ).toMatchObject({ bytesDefinition: 'deployable', gate: 'pass' })
  })

  it('names the rows the gate failed', () => {
    const run = {
      ...makeRun(SHA_A, {
        tailwind: measured(10),
        'devup-ui': measured(12),
      }),
      run: { id: '7', commit: SHA_A, devupUi: '1.0.45' },
    }

    const entry = historyEntry(run, baseline, '2026-11-14T00:00:00.000Z')

    expect(entry).toMatchObject({
      gate: 'fail',
      regressions: ['devup-ui'],
      measuredAt: null,
      devupUi: '1.0.45',
    })
  })

  const entry = (sha: string, committedAt: string) => ({
    sha,
    committedAt,
    rows: {},
  })

  it('starts a history and keeps it in commit order', () => {
    const first = addToHistory(undefined, entry('b', '2026-11-02'))
    const second = addToHistory(first, entry('a', '2026-11-01'))

    expect(second).toEqual({
      columns: HISTORY_COLUMNS,
      entries: [entry('a', '2026-11-01'), entry('b', '2026-11-02')],
    })
  })

  it('replaces the entry of a re-run commit', () => {
    const history = addToHistory(undefined, entry('a', '2026-11-01'))
    const rerun = { ...entry('a', '2026-11-01'), run: 'again' }

    expect(addToHistory(history, rerun).entries).toEqual([rerun])
  })

  it('keeps only the latest entries', () => {
    let history = addToHistory(undefined, entry('a', '2026-11-01'))
    history = addToHistory(history, entry('b', '2026-11-02'), 1)

    expect(history.entries).toEqual([entry('b', '2026-11-02')])
    expect(HISTORY_LIMIT).toBeGreaterThan(100)
  })

  it('keeps the checked history in the shape the refresh writes', () => {
    expect(storedHistory.columns).toEqual(HISTORY_COLUMNS)
    expect(storedHistory.entries.map((kept) => kept.sha)).toContain(
      storedResults.run.sha,
    )
    for (const kept of storedHistory.entries)
      for (const row of Object.values(kept.rows))
        expect(row).toHaveLength(HISTORY_COLUMNS.length)
  })

  it('prints the trend of a row against its control', () => {
    const history = {
      entries: [
        {
          sha: SHA_A,
          committedAt: '2026-11-01',
          gate: 'pass',
          rows: {
            tailwind: [10, 9, 11, 1000, 100, null],
            'devup-ui': [9, 8, 10, 900, 50, 400],
          },
        },
        {
          sha: SHA_B,
          committedAt: '2026-11-02',
          gate: 'fail',
          bytesDefinition: 'deployable',
          rows: { 'devup-ui': [12, 11, 13, 90, 50, null] },
        },
        { sha: 'c'.repeat(40), committedAt: '2026-11-03', rows: {} },
      ],
    }

    expect(trend(history, 'devup-ui', 'tailwind')).toBe(
      [
        'aaaaaaaa 2026-11-01 pass 9.00s 0.900x tailwind, 900 output-dir bytes, css 50, client 400',
        'bbbbbbbb 2026-11-02 fail 12.00s, 90 deployable bytes, css 50',
      ].join('\n'),
    )
  })
})

describe('benchmark refresh order', () => {
  const ancestry =
    (pairs: Record<string, boolean | null>) => (a: string, b: string) =>
      pairs[`${a}>${b}`] ?? null

  it('skips a commit that is not on main', () => {
    expect(
      refreshPlan(
        { measured: 'x', head: 'h', published: 'p' },
        ancestry({ 'x>h': false }),
      ),
    ).toEqual({ history: false, results: false, reason: 'not on main' })
  })

  it('records but does not publish a commit older than the published one', () => {
    expect(
      refreshPlan(
        { measured: 'x', head: 'h', published: SHA_B },
        ancestry({ 'x>h': true, [`${SHA_B}>x`]: false }),
      ),
    ).toEqual({
      history: true,
      results: false,
      reason: 'the published results come from a later commit (bbbbbbbb)',
    })
  })

  it.each([
    ['a newer commit', SHA_A, true],
    ['an unknown published commit', SHA_A, null],
    ['no published commit', undefined, null],
  ])('publishes %s', (_, published, answer) => {
    expect(
      refreshPlan(
        { measured: 'x', head: 'h', published },
        ancestry({ 'x>h': true, [`${published}>x`]: answer }),
      ),
    ).toEqual({
      history: true,
      results: true,
      reason: 'newer or the same commit',
    })
  })
})

describe('benchmark JSON', () => {
  it('prints short arrays on one line, fills long numbers and splits long strings', () => {
    const long = Array.from({ length: 12 }, (_, index) => 1_000_000 + index)
    const longNames = Array.from(
      { length: 6 },
      (_, index) => `target-${index}-xxxxxxxx`,
    )

    // As Prettier prints them, so `bun run lint` accepts what the refresh writes
    expect(
      formatJson({
        short: [1, 2.5, null],
        names: ['a', 'b'],
        long,
        longNames,
        empty: [],
      }),
    ).toBe(
      [
        '{',
        '  "short": [1, 2.5, null],',
        '  "names": ["a", "b"],',
        '  "long": [',
        '    1000000, 1000001, 1000002, 1000003, 1000004, 1000005, 1000006, 1000007,',
        '    1000008, 1000009, 1000010, 1000011',
        '  ],',
        '  "longNames": [',
        ...longNames.map(
          (name, index) =>
            `    "${name}"${index === longNames.length - 1 ? '' : ','}`,
        ),
        '  ],',
        '  "empty": []',
        '}',
        '',
      ].join('\n'),
    )
  })

  it('fits a last array of 80 columns on one line, as it takes no comma', () => {
    // Exactly 80 columns, 81 when a comma follows
    const values = [1.5, 2.25, 3.125, 38662174, 1512, 571629, 12345678, 1234]
    const fits = { aaaaaaaaaaaaaaaa: values }
    const split = { aaaaaaaaaaaaaaaa: values, b: 1 }
    const line = `  "aaaaaaaaaaaaaaaa": [${values.join(', ')}]`

    expect(line).toHaveLength(80)
    expect(formatJson(fits)).toBe(`{\n${line}\n}\n`)
    expect(formatJson(split).split('\n')[1]).toBe('  "aaaaaaaaaaaaaaaa": [')
  })

  it('formats the checked files exactly as it writes them', () => {
    for (const file of ['benchmark-results.json', 'benchmark/history.json']) {
      const text = readFileSync(join(ROOT, file), 'utf8').replaceAll(
        '\r\n',
        '\n',
      )
      expect(formatJson(JSON.parse(text))).toBe(text)
    }
  })
})

describe('benchmark refresh CLI', () => {
  let directory: string

  const git = (...args: string[]) =>
    spawnSync(
      'git',
      ['-c', 'user.name=test', '-c', 'user.email=test@test', ...args],
      { cwd: directory, encoding: 'utf8' },
    ).stdout.trim()
  const commit = (message: string, date: string) => {
    spawnSync(
      'git',
      [
        '-c',
        'user.name=test',
        '-c',
        'user.email=test@test',
        'commit',
        '--allow-empty',
        '-q',
        '-m',
        message,
      ],
      {
        cwd: directory,
        env: {
          ...process.env,
          GIT_COMMITTER_DATE: date,
          GIT_AUTHOR_DATE: date,
        },
      },
    )
    return git('rev-parse', 'HEAD')
  }
  const write = (file: string, value: unknown) =>
    writeFileSync(join(directory, file), JSON.stringify(value))
  const read = (file: string) =>
    JSON.parse(readFileSync(join(directory, file), 'utf8'))
  const refresh = (...args: string[]) =>
    spawnSync(process.execPath, [join(ROOT, 'benchmark-refresh.js'), ...args], {
      cwd: directory,
      encoding: 'utf8',
      timeout: 20_000,
    })

  let first: string
  let second: string

  beforeEach(() => {
    directory = mkdtempSync(join(tmpdir(), 'devup-benchmark-refresh-'))
    git('init', '-q')
    write('benchmark-baseline.json', { samples: 3, rows: {} })
    git('add', '.')
    first = commit('first', '2026-11-01T00:00:00Z')
    second = commit('second', '2026-11-02T00:00:00Z')
    write('benchmark-manifest.json', {
      targets: [{ name: 'tailwind', group: 'webpack' }],
      controls: { webpack: 'tailwind' },
    })
  })

  afterEach(() => {
    rmSync(directory, { recursive: true, force: true })
  })

  it('publishes and records a newer commit, then prints its trend', () => {
    write('benchmark-results.json', results)
    write('benchmark-run.json', makeRun(second, { tailwind: measured(10) }))

    const result = refresh()

    expect(result.status).toBe(0)
    expect(read('benchmark-results.json').run.sha).toBe(second)
    expect(read('benchmark/history.json').entries).toMatchObject([
      { sha: second, committedAt: '2026-11-02T00:00:00.000Z', gate: 'pass' },
    ])

    const printed = refresh('--trend', 'tailwind')
    expect(printed.status).toBe(0)
    expect(printed.stdout).toContain(
      `${second.slice(0, 8)} 2026-11-02T00:00:00.000Z pass 10.00s 1.000x tailwind`,
    )
  })

  it('records an older commit without replacing the published results', () => {
    write('benchmark-results.json', {
      ...results,
      run: { ...results.run, sha: second },
    })
    write('benchmark-run.json', makeRun(first, { tailwind: measured(10) }))

    const result = refresh()

    expect(result.status).toBe(0)
    expect(read('benchmark-results.json').run.sha).toBe(second)
    expect(read('benchmark/history.json').entries).toMatchObject([
      { sha: first },
    ])
  })

  it('falls back to the checked baseline when the commit has none', () => {
    rmSync(join(directory, 'benchmark-baseline.json'))
    git('rm', '-q', '--cached', 'benchmark-baseline.json')
    const third = commit('third', '2026-11-03T00:00:00Z')
    write('benchmark-baseline.json', { samples: 3, rows: {} })
    write('benchmark-results.json', results)
    write('benchmark-run.json', makeRun(third, { tailwind: measured(10) }))

    const result = refresh()

    expect(result.status).toBe(0)
    expect(read('benchmark/history.json').entries).toHaveLength(1)
  })

  it('leaves everything alone for a commit that is not on main', () => {
    write('benchmark-results.json', results)
    write('benchmark-run.json', makeRun(SHA_B, { tailwind: measured(10) }))

    const result = refresh()

    expect(result.status).toBe(0)
    expect(result.stdout).toContain('not on main')
    expect(existsSync(join(directory, 'benchmark/history.json'))).toBe(false)
    expect(read('benchmark-results.json')).toEqual(results)
  })

  it.each([
    [['--trend', 'missing'], 'benchmark-manifest.json has no missing'],
    [[], 'benchmark-run.json names no commit'],
  ])('rejects %p', (args, message) => {
    write('benchmark-results.json', results)
    write('benchmark-run.json', makeRun('', {}))

    const result = refresh(...args)

    expect(result.status).not.toBe(0)
    expect(result.stderr).toContain(message)
  })
})
