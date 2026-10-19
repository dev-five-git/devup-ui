import { execFileSync } from 'node:child_process'
import { appendFileSync, readFileSync } from 'node:fs'

function requireInput(condition, message) {
  if (!condition) throw new Error(`resolve-wasm-size-sources: ${message}`)
}

export function gitEnvironment(environment) {
  // This query needs no repository, even when inherited metadata is foreign.
  const localNames = new Set(
    execFileSync('git', ['rev-parse', '--local-env-vars'], {
      env: environment,
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'pipe'],
    })
      .trim()
      .split(/\r?\n/),
  )
  return Object.fromEntries(
    Object.entries(environment).filter(
      ([name]) => !localNames.has(name.toUpperCase()),
    ),
  )
}

function git(cwd, args) {
  return execFileSync('git', ['--no-replace-objects', ...args], {
    cwd,
    env: gitEnvironment(process.env),
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
  }).trim()
}

function commit(cwd, sha) {
  requireInput(
    typeof sha === 'string' && /^[a-f0-9]{40}(?![\s\S])/.test(sha),
    'expected an exact 40-character commit ID',
  )
  requireInput(
    git(cwd, ['cat-file', '-t', sha]) === 'commit',
    `${sha} is not a commit`,
  )
  return sha
}

export function resolveSources(input, cwd) {
  requireInput(
    input !== null && typeof input === 'object' && !Array.isArray(input),
    'missing source input',
  )
  switch (input.eventName) {
    case 'pull_request': {
      const candidate = commit(cwd, input.sha)
      const header = git(cwd, ['cat-file', '-p', candidate]).split('\n\n', 1)[0]
      const parent = header.match(/^parent ([a-f0-9]{40})$/m)?.[1]
      requireInput(parent, `${candidate} has no first parent`)
      return { base: commit(cwd, parent), candidate }
    }
    case 'push':
      return {
        base: commit(cwd, input.event?.before),
        candidate: commit(cwd, input.event?.after),
      }
    default:
      throw new Error('resolve-wasm-size-sources: unsupported event')
  }
}

export function cli(environment) {
  for (const name of [
    'GITHUB_EVENT_PATH',
    'GITHUB_WORKSPACE',
    'GITHUB_ENV',
    'GITHUB_STEP_SUMMARY',
  ])
    requireInput(
      typeof environment[name] === 'string' && environment[name].trim(),
      `required ${name}`,
    )
  const sources = resolveSources(
    {
      eventName: environment.GITHUB_EVENT_NAME,
      sha: environment.GITHUB_SHA,
      event: JSON.parse(readFileSync(environment.GITHUB_EVENT_PATH, 'utf8')),
    },
    environment.GITHUB_WORKSPACE,
  )
  const assignments = `BASE_SHA=${sources.base}\nCANDIDATE_SHA=${sources.candidate}\n`
  appendFileSync(
    environment.GITHUB_STEP_SUMMARY,
    `## WASM size sources\n\nBase: \`${sources.base}\`\n\nCandidate: \`${sources.candidate}\`\n\n`,
  )
  appendFileSync(environment.GITHUB_ENV, assignments)
  console.info(assignments.trim())
  return 0
}
