import { mkdirSync, writeFileSync } from 'node:fs'
import { IncomingMessage } from 'node:http'
import { Socket } from 'node:net'
import { join } from 'node:path'

import { afterEach, describe, expect, it } from 'bun:test'

import { startCoordinator } from '../coordinator'
import { requestCoordinator } from '../coordinator-client'
import { assertOwnership, HttpError } from '../coordinator-http'
import { parsePortFile } from '../coordinator-port'
import {
  createTestApp,
  http,
  ownership,
  removeTestApps,
} from './coordinator-app'

function unicodeCoordinator() {
  const app = createTestApp()
  const root = join(app.root, '테스트-app')
  mkdirSync(root)
  const identity = { ...app.identity, project: root }
  const handle = startCoordinator(
    app.options({ projectRoot: root, identity, singleCss: true, watch: true }),
  )
  const operation = {
    portFile: app.portFile,
    identity,
    resourcePath: join(root, 'App.tsx'),
    path: '/css',
    timeoutMs: 2000,
  }
  return { handle, operation }
}

afterEach(removeTestApps)

describe('encoded coordinator HTTP ownership', () => {
  it('extracts native classes matching CSS when the real coordinator root is Unicode', async () => {
    const { handle, operation } = unicodeCoordinator()
    try {
      await handle.ready
      const code =
        'import { Box } from "@devup-ui/react"; export const App = <Box color="red" />'
      writeFileSync(operation.resourcePath, code)

      const extracted = await requestCoordinator({
        ...operation,
        path: '/extract',
        method: 'POST',
        body: JSON.stringify({
          filename: 'App.tsx',
          resourcePath: operation.resourcePath,
          code,
        }),
      })
      const css = await requestCoordinator(operation)

      expect(extracted).toContain('<div')
      const className = /className=\\"([^\\]+)\\"/.exec(extracted)?.[1]
      expect(className).toBeDefined()
      expect(css).toContain(`.${className}{color:red}`)
    } finally {
      await handle.drain()
    }
  })
  it('preserves the real project path in JSON when HTTP ownership is encoded', async () => {
    const { handle, operation } = unicodeCoordinator()
    try {
      await handle.ready

      const health = await requestCoordinator({ ...operation, path: '/health' })

      expect(parsePortFile(health).project).toBe(operation.identity.project)
    } finally {
      await handle.drain()
    }
  })
  it.each(['project', 'token', 'double-encoded project'] as const)(
    'rejects %s ownership when the real coordinator root is Unicode',
    async (field) => {
      const { handle, operation } = unicodeCoordinator()
      try {
        await handle.ready
        const endpoint = parsePortFile(
          await requestCoordinator({ ...operation, path: '/health' }),
        )
        const headers = {
          ...ownership(operation.identity),
          ...{
            project: {
              'x-devup-project': encodeURIComponent(
                join(operation.identity.project, 'other'),
              ),
            },
            token: { 'x-devup-token': 'wrong-token' },
            'double-encoded project': {
              'x-devup-project': encodeURIComponent(
                encodeURIComponent(operation.identity.project),
              ),
            },
          }[field],
        }

        const reply = await http(endpoint.port, 'GET', '/css', { headers })

        expect(reply.status).toBe(403)
      } finally {
        await handle.drain()
      }
    },
  )
  it('rejects raw project headers instead of accepting a legacy fallback', () => {
    const req = new IncomingMessage(new Socket())
    const identity = { project: '/app', token: 'token' }
    req.headers = { 'x-devup-project': '/app', 'x-devup-token': 'token' }

    expect(() => assertOwnership(req, '/css', identity, true)).toThrow(
      HttpError,
    )
  })
})
