import { execFileSync, spawnSync } from 'node:child_process'
import { randomUUID } from 'node:crypto'
import {
  closeSync,
  openSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { dirname, join, relative, resolve, sep } from 'node:path'
import { fileURLToPath } from 'node:url'

import { aliasSource } from './W72-real-alias.mjs'
import { collectResults } from './W72-real-collection.mjs'
import {
  resolveCli,
  snapshot,
  validateCheckout,
} from './W72-real-environment.mjs'
import {
  captureGenerated,
  inventory,
  json,
  prepareCaches,
  restoreFiles,
  sha,
} from './W72-real-files.mjs'

const home = resolve(
  process.env.W72_OUT ?? dirname(fileURLToPath(import.meta.url)),
)
const root = resolve(process.env.W72_ROOT ?? process.cwd())
const [app, variant, repetition, ...extra] = process.argv.slice(2)
if (
  !['landing', 'vite'].includes(app) ||
  !['W1', 'W2', 'N1'].includes(variant) ||
  !/^\d+$/.test(repetition ?? '') ||
  extra.length
) {
  throw new Error(
    'Usage: bun W72-real-build.mjs <landing|vite> <W1|W2|N1> <repetition>',
  )
}
const id = `W72-real-${app}-${variant}-${repetition}-${Date.now()}-${process.pid}-${randomUUID()}`
const prefix = join(home, id)
const node = join(home, 'W72-node', 'node.exe')
const binding = join(
  home,
  `W72-${variant}`,
  variant === 'N1' ? 'index.cjs' : 'index.js',
)
const alias = join(root, 'bindings/devup-ui-wasm/pkg/index.js')
const cwd = join(root, 'apps', app)
const env = {
  ...process.env,
  GIT_MASTER: '1',
  W72_BINDING: binding,
  W72_REAL_PREFIX: prefix,
}
for (const key of Object.keys(env))
  if (key.toLowerCase() === 'path') delete env[key]
env.PATH = `${dirname(node)}${sep === '\\' ? ';' : ':'}${process.env.PATH ?? process.env.Path ?? ''}`
const git = (args) =>
  execFileSync('git.exe', ['-C', root, ...args], {
    env,
    encoding: 'utf8',
    windowsHide: true,
  })
const ps = (code) =>
  execFileSync(
    'powershell.exe',
    ['-NoProfile', '-NonInteractive', '-Command', code],
    { env, encoding: 'utf8', windowsHide: true },
  )
const result = {
  id,
  app,
  variant,
  repetition,
  root,
  out: home,
  cwd,
  binding,
  runnerPid: process.pid,
  parentPid: process.ppid,
  commands: [],
  baseline: [],
  startedAt: new Date().toISOString(),
  watchdog: 'External W72-watch.ps1 -RealBuild required; 6 GiB tree cap',
}
let original,
  lock,
  log,
  installed = false
/** @type {({ readonly path: string, readonly exists: true, readonly bytes: Buffer } | { readonly path: string, readonly exists: false })[]} */
const generated = []

function build(args) {
  writeFileSync(log, `\n=== ${node} ${JSON.stringify(args)} ===\n`)
  const start = performance.now()
  const child = spawnSync(node, args, {
    cwd,
    env,
    stdio: ['ignore', log, log],
    windowsHide: true,
  })
  result.commands.push({
    executable: node,
    args,
    pid: child.pid,
    status: child.status,
    signal: child.signal,
    wallMs: performance.now() - start,
    error: child.error?.message,
  })
  if (child.error) throw child.error
  if (child.status !== 0)
    throw new Error(
      `Build stage failed: status=${child.status}, signal=${child.signal}`,
    )
}

function cleanupChildren() {
  const text = ps(
    `$ErrorActionPreference='Stop'; $since=[DateTimeOffset]::Parse('${result.startedAt}').LocalDateTime; $all=@(Get-CimInstance Win32_Process | Where-Object { $_.ProcessId -eq ${process.pid} -or $_.CreationDate -ge $since }); $birth=@{}; foreach($p in $all) { $birth[[int]$p.ProcessId]=$p.CreationDate }; $known=[Collections.Generic.HashSet[int]]::new(); [void]$known.Add(${process.pid}); do { $added=0; foreach($p in $all) { $parent=[int]$p.ParentProcessId; if($known.Contains($parent) -and $birth.ContainsKey($parent) -and $p.CreationDate -ge $birth[$parent] -and $known.Add([int]$p.ProcessId)) { $added++ } } } while($added -gt 0); $remaining=@($all | Where-Object { $_.ProcessId -ne ${process.pid} -and $_.Name -in @('node.exe','bun.exe') -and ($known.Contains([int]$_.ProcessId) -or ($_.CommandLine -and $_.CommandLine.Replace([char]92,[char]47).ToLowerInvariant().Contains('${root.replaceAll('\\', '/').replaceAll("'", "''").toLowerCase()}'))) }); ConvertTo-Json -InputObject @($remaining | Select-Object ProcessId,ParentProcessId,CreationDate,CommandLine) -Compress`,
  )
  result.cleanedChildren = []
  result.remainingChildren = JSON.parse(text)
  if (result.remainingChildren.length)
    throw new Error(
      'Build children remain; inspect ownership before stopping them',
    )
}

try {
  result.environmentBefore = snapshot(ps)
  if (
    result.environmentBefore.freeBytes < 7 * 1024 ** 3 ||
    Number(result.environmentBefore.windows.memory.FreePhysicalMemory) * 1024 <
      7 * 1024 ** 3
  )
    throw new Error('Real build requires at least 7 GiB free physical memory')
  validateCheckout({ root, cwd, app }, git)
  result.head = git(['rev-parse', 'HEAD']).trim()
  result.nodeVersion = execFileSync(node, ['--version'], {
    env,
    encoding: 'utf8',
  }).trim()
  lock = openSync(join(home, 'W72-real-build.lock'), 'wx')
  writeFileSync(
    lock,
    JSON.stringify({
      pid: process.pid,
      id,
      alias,
      backup: `${prefix}-original-index.bin`,
    }),
  )
  if (git(['ls-files', '--', relative(root, alias)]).trim())
    throw new Error('Binding alias must not be tracked')
  git(['check-ignore', '--', relative(root, alias)])
  original = readFileSync(alias)
  if (original.includes('W72_REAL_ALIAS'))
    throw new Error('Stale W72 alias: recover its backup before running')
  const names = [
    ...original.toString().matchAll(/exports\.([A-Za-z_$][\w$]*)\s*=/g),
  ].map((match) => match[1])
  if (!names.includes('codeExtract') || !names.includes('Output'))
    throw new Error('Unrecognized generated WASM baseline')
  result.bindingInputs = inventory(dirname(binding), root)
  const aliasPackage = JSON.parse(
    readFileSync(join(dirname(alias), 'package.json'), 'utf8'),
  )
  if (aliasPackage.type === 'module')
    throw new Error('Generated alias must be CommonJS')
  const cli = resolveCli({ app, cwd, binding, root }, result)
  captureGenerated(cwd, app, generated)
  prepareCaches({ root, cwd, env, git }, result.baseline)
  json(`${prefix}-baseline.json`, result.baseline)
  writeFileSync(`${prefix}-original-index.bin`, original, { flag: 'wx' })
  json(
    `${prefix}-generated-baseline.json`,
    generated.map((file) =>
      file.exists
        ? {
            filename: file.path,
            exists: true,
            base64: file.bytes.toString('base64'),
            sha256: sha(file.bytes),
          }
        : { filename: file.path, exists: false },
    ),
  )
  result.originalAliasSha256 = sha(original)
  for (const cache of result.baseline)
    rmSync(join(root, cache.directory), { recursive: true })
  log = openSync(`${prefix}.log`, 'wx')
  installed = true
  writeFileSync(alias, aliasSource([...new Set(names)]))
  const start = performance.now()
  try {
    if (app === 'landing') build([join(cwd, 'script.js')])
    build([cli, 'build'])
  } finally {
    result.wallMs = performance.now() - start
  }
  result.status = 'success'
} catch (error) {
  result.status = 'failed'
  result.error =
    error instanceof Error
      ? { message: error.message, stack: error.stack }
      : { message: String(error) }
  process.exitCode = 1
} finally {
  try {
    if (installed) cleanupChildren()
  } catch (error) {
    result.cleanupError = String(error)
    process.exitCode = 1
  }
  try {
    restoreFiles({ installed, alias, original, generated }, result)
  } catch (error) {
    result.restoreError = String(error)
    process.exitCode = 1
  }
  if (log !== undefined) closeSync(log)
  try {
    result.environmentAfter = snapshot(ps)
    collectResults({ installed, app, cwd, root, home, id }, result)
  } catch (error) {
    result.collectionError = String(error)
    process.exitCode = 1
  }
  if (process.exitCode) result.status = 'failed'
  result.finishedAt = new Date().toISOString()
  json(`${prefix}.json`, result)
  if (lock !== undefined) {
    closeSync(lock)
    if (!result.restoreError) rmSync(join(home, 'W72-real-build.lock'))
  }
  process.stdout.write(
    JSON.stringify({
      json: `${prefix}.json`,
      log: `${prefix}.log`,
      status: result.status,
      wallMs: result.wallMs,
      insideDevupMs: result.insideDevupMs,
    }) + '\n',
  )
}
