import assert from 'node:assert/strict'
import { spawn } from 'node:child_process'
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { createServer } from 'node:http'
import { join } from 'node:path'

import { hashFile, writeJson } from './provenance.mjs'

// Async children are essential: a blocking install would starve this server.
export async function child(program, args, options = {}) {
  return new Promise((resolve, reject) => {
    const subprocess = spawn(program, args, {
      env: { ...process.env, NODE_TEST_CONTEXT: undefined },
      ...options,
      stdio: ['ignore', 'pipe', 'pipe'],
      timeout: 180000,
    })
    let stdout = ''
    let stderr = ''
    subprocess.stdout.setEncoding('utf8').on('data', (chunk) => {
      stdout += chunk
    })
    subprocess.stderr.setEncoding('utf8').on('data', (chunk) => {
      stderr += chunk
    })
    subprocess.once('error', reject)
    subprocess.once('close', (code, signal) => {
      if (code === 0) resolve({ stdout, stderr })
      else
        reject(
          Object.assign(new Error(`${program} exited ${code}/${signal}`), {
            code,
            stdout,
            stderr,
          }),
        )
    })
  })
}

// Each real install has a new fixture, empty caches and no lifecycle authority.
export async function installConsumer(options) {
  const fixture = join(
    options.directory,
    `${options.manager}-${options.omit ? 'omitted' : 'selected'}`,
  )
  mkdirSync(fixture)
  writeJson(join(fixture, 'package.json'), {
    name: 'clean-native-consumer',
    private: true,
    dependencies: { '@devup-ui/binding': options.version },
  })
  const env = {
    ...process.env,
    NODE_PATH: '',
    npm_config_userconfig: join(fixture, 'empty.npmrc'),
    npm_config_globalconfig: join(fixture, 'global.npmrc'),
    npm_config_cache: join(fixture, 'cache'),
    npm_config_ignore_scripts: 'true',
    npm_config_registry: options.origin,
    BUN_INSTALL_CACHE_DIR: join(fixture, 'cache'),
  }
  writeFileSync(env.npm_config_userconfig, '')
  writeFileSync(env.npm_config_globalconfig, '')
  const args = [
    'install',
    '--ignore-scripts',
    '--registry',
    options.origin,
    ...(options.manager === 'npm' ? ['--no-audit', '--no-fund'] : []),
    ...(options.omit ? ['--omit=optional'] : []),
  ]
  const output =
    options.manager === 'npm'
      ? await child(process.execPath, [options.npmCli, ...args], {
          cwd: fixture,
          env,
        })
      : await child('bun', args, { cwd: fixture, env })
  writeJson(join(fixture, 'install.json'), output)
  return { fixture, env }
}
export async function registry(bundle, directory) {
  const requests = []
  const bytes = new Map(
    bundle.packs.map((pack) => {
      const file = join(directory, pack.file)
      assert.equal(hashFile(file), pack.sha256, 'registry tarball hash')
      return [pack.file, readFileSync(file)]
    }),
  )
  let origin
  const server = createServer((request, response) => {
    const path = decodeURIComponent(
      new URL(request.url, 'http://localhost').pathname,
    )
    requests.push({ method: request.method, path })
    const pack = bundle.packs.find(
      (item) => path === `/${item.name}` || path === `/tarballs/${item.file}`,
    )
    if (request.method !== 'GET' || !pack) {
      response.writeHead(404).end('not in verified bundle')
      return
    }
    if (path.startsWith('/tarballs/')) {
      response
        .writeHead(200, { 'content-type': 'application/octet-stream' })
        .end(bytes.get(pack.file))
    } else {
      const version = {
        ...pack.manifest,
        dist: {
          tarball: `${origin}/tarballs/${pack.file}`,
          integrity: pack.integrity,
        },
      }
      response.writeHead(200, { 'content-type': 'application/json' }).end(
        JSON.stringify({
          name: pack.name,
          'dist-tags': { latest: pack.version },
          versions: { [pack.version]: version },
        }),
      )
    }
  })
  await new Promise((resolve, reject) => {
    server.once('error', reject)
    server.listen(0, '127.0.0.1', resolve)
  })
  origin = `http://127.0.0.1:${server.address().port}`
  return {
    origin,
    requests,
    close: () =>
      new Promise((resolve, reject) => {
        server.closeAllConnections()
        server.close((error) => (error ? reject(error) : resolve()))
      }),
  }
}
