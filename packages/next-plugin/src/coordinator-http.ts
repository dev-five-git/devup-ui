import type { IncomingMessage, ServerResponse } from 'node:http'

import { locatedError } from './coordinator-engine'
import type { CoordinatorIdentity } from './coordinator-port'

export class HttpError extends Error {
  constructor(
    readonly status: number,
    message: string,
  ) {
    super(message)
  }
}

export interface ExtractRequest {
  readonly filename: string
  readonly code: string
  readonly resourcePath: string
}

export function readBody(req: IncomingMessage): Promise<string> {
  return new Promise((resolve, reject) => {
    const chunks: Buffer[] = []
    req.on('data', (chunk: Buffer) => chunks.push(chunk))
    req.on('end', () => resolve(Buffer.concat(chunks).toString('utf-8')))
    req.on('error', reject)
  })
}

export function sendJson(
  res: ServerResponse,
  status: number,
  body: unknown,
): void {
  res.writeHead(status, { 'Content-Type': 'application/json' })
  res.end(JSON.stringify(body))
}

/**
 * Refuse a request that names another project or token. With `enforce`, the
 * headers are required too; without it they are checked only when sent.
 */
export function assertOwnership(
  req: IncomingMessage,
  pathname: string,
  identity: CoordinatorIdentity,
  enforce: boolean,
): void {
  const project = req.headers['x-devup-project']
  const token = req.headers['x-devup-token']
  const missing = project === undefined || token === undefined
  const foreign =
    (project !== undefined &&
      project !== encodeURIComponent(identity.project)) ||
    (token !== undefined && token !== identity.token)
  if ((enforce && missing) || foreign) {
    throw new HttpError(
      403,
      `${pathname}:1:1: devup-ui coordinator cannot accept this request: it does not carry this build's project and token. Fix: read the port file the plugin published for this build and send its x-devup-project and x-devup-token headers.`,
    )
  }
}

export function parseExtractRequest(body: string): ExtractRequest {
  let data: unknown
  try {
    data = JSON.parse(body)
  } catch (cause) {
    throw new HttpError(
      400,
      locatedError(
        '/extract',
        'read the request body',
        cause,
        'POST JSON {filename, code, resourcePath}.',
      ).message,
    )
  }
  if (
    typeof data !== 'object' ||
    data === null ||
    !('filename' in data) ||
    typeof data.filename !== 'string' ||
    !('code' in data) ||
    typeof data.code !== 'string' ||
    !('resourcePath' in data) ||
    typeof data.resourcePath !== 'string'
  ) {
    throw new HttpError(
      400,
      locatedError(
        '/extract',
        'read the request body',
        'filename, code and resourcePath must all be strings',
        'POST JSON {filename, code, resourcePath}.',
      ).message,
    )
  }
  return {
    filename: data.filename,
    code: data.code,
    resourcePath: data.resourcePath,
  }
}

/** The `fileNum` query value, or undefined when it is absent. */
export function parseFileNum(value: string | null): number | undefined {
  if (value === null) return undefined
  if (!/^\d+$/.test(value)) {
    throw new HttpError(
      400,
      locatedError(
        '/css',
        'read the request',
        `fileNum "${value}" is not a number`,
        'request /css?fileNum=<integer>.',
      ).message,
    )
  }
  return Number(value)
}
