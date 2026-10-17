import { createReadStream } from 'node:fs'

/** Relay cancellation without handing a foreign-realm signal to native fs. */
export async function readWebpackLoaderBytes(
  filename: string,
  signal: AbortSignal,
): Promise<Buffer> {
  signal.throwIfAborted()
  const stream = createReadStream(filename)
  const abort = () => stream.destroy()
  signal.addEventListener('abort', abort, { once: true })
  const chunks: Buffer[] = []
  try {
    for await (const chunk of stream) {
      signal.throwIfAborted()
      if (!Buffer.isBuffer(chunk))
        throw new TypeError('native loader read returned non-buffer bytes')
      chunks.push(chunk)
    }
    signal.throwIfAborted()
    return Buffer.concat(chunks)
  } catch (error) {
    signal.throwIfAborted()
    throw error
  } finally {
    signal.removeEventListener('abort', abort)
    stream.destroy()
  }
}
