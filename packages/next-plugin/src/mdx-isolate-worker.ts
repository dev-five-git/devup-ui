import { type MessagePort, parentPort, workerData } from 'node:worker_threads'

import {
  type MdxIsolateControl,
  parseMdxIsolateControl,
  parseMdxIsolateDemand,
} from './mdx-isolate-protocol'
import { createMdxIsolateRuntime } from './mdx-isolate-runtime'
import { isMdxRecord } from './mdx-pipeline'

export function startMdxIsolateWorker(port: MessagePort, root: unknown) {
  if (typeof root !== 'string')
    throw new TypeError(
      'dormant MDX worker requires its owning channel and project root',
    )
  const compile = createMdxIsolateRuntime(root)
  let tail = Promise.resolve()
  port.on('message', (value: unknown) => {
    tail = tail.then(async () => {
      if (isMdxRecord(value) && value.operation !== undefined) {
        const request = parseMdxIsolateControl(value)
        const handlers = {
          check: () => {
            const verdict = compile.check(request.run, request.filename)
            port.postMessage(
              verdict.error
                ? { ...verdict, terminal: compile.quarantine(verdict.error) }
                : verdict,
            )
          },
          retire: () => {
            compile.retire(request.run)
            port.postMessage({ run: request.run, filename: request.filename })
          },
          finalize: () => {
            port.postMessage({
              run: request.run,
              filename: request.filename,
              terminal: compile.quarantine({
                filename: request.filename,
                message: 'parent sealed native work',
                line: 1,
                column: 1,
              }),
            })
          },
        } satisfies Record<MdxIsolateControl['operation'], () => void>
        handlers[request.operation]()
        return
      }
      const request = parseMdxIsolateDemand(value)
      const outcome = await compile(request)
      port.postMessage(outcome)
    })
  })
}
if (parentPort) startMdxIsolateWorker(parentPort, workerData)
