import { expect, it } from 'bun:test'

import { CoverageError } from '../../../../test-harness/lcov'
import { object } from '../../../../test-harness/producer-data'
import {
  connectProducer,
  type ProducerProtocol,
} from '../../../../test-harness/producer-protocol'

type WireMode = 'success' | 'rejected' | 'malformed' | 'silent' | 'closed'

function wire(mode: WireMode) {
  return Bun.serve({
    hostname: '127.0.0.1',
    port: 0,
    fetch(request, server) {
      if (server.upgrade(request)) return undefined
      return new Response('upgrade required', { status: 400 })
    },
    websocket: {
      message(socket, data) {
        const request = object(JSON.parse(String(data)))
        if (request['method'] !== 'Probe') {
          socket.send(JSON.stringify({ id: request['id'], result: {} }))
          return
        }
        switch (mode) {
          case 'success':
            socket.send(
              JSON.stringify({ method: 'Console.messagesCleared', params: {} }),
            )
            socket.send(
              JSON.stringify({ id: request['id'], result: { value: 42 } }),
            )
            return
          case 'rejected':
            socket.send(
              JSON.stringify({
                id: request['id'],
                error: { code: -32601, message: 'unsupported' },
              }),
            )
            return
          case 'malformed':
            socket.send('not-json')
            return
          case 'silent':
            return
          case 'closed':
            socket.close()
            return
          default: {
            const exhaustive: never = mode
            throw new Error(`unreachable wire mode ${exhaustive}`)
          }
        }
      },
    },
  })
}

it('returns only the matched RPC response when a real wire also emits an event', async () => {
  const server = wire('success')
  let protocol: ProducerProtocol | undefined
  try {
    protocol = await connectProducer(`ws://127.0.0.1:${server.port}`)
    const result = await protocol.request('Probe')
    expect(result).toEqual({ value: 42 })
    expect(
      protocol.events.some(
        (event) => event['method'] === 'Console.messagesCleared',
      ),
    ).toBe(true)
  } finally {
    protocol?.close()
    await server.stop(true)
  }
})

it.each(['rejected', 'malformed', 'silent', 'closed'] as const)(
  'rejects the real %s transport response instead of emitting trusted producer data',
  async (mode) => {
    const server = wire(mode)
    let protocol: ProducerProtocol | undefined
    try {
      protocol = await connectProducer(`ws://127.0.0.1:${server.port}`)
      await expect(protocol.request('Probe')).rejects.toThrow(Error)
    } finally {
      protocol?.close()
      await server.stop(true)
    }
  },
  30000,
)

it('rejects actual connection refusal when the endpoint has stopped', async () => {
  const server = wire('success')
  const url = `ws://127.0.0.1:${server.port}`
  await server.stop(true)
  await expect(connectProducer(url)).rejects.toThrow(CoverageError)
})
