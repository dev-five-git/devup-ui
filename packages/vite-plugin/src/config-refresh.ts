import { existsSync } from 'node:fs'
import { dirname } from 'node:path'

export function watchConfigFiles(
  paths: readonly string[],
  add: (path: string) => void,
): void {
  for (const path of paths) {
    add(path.replaceAll('\\', '/'))
    if (existsSync(path)) continue
    let ancestor = dirname(path)
    while (!existsSync(ancestor) && dirname(ancestor) !== ancestor)
      ancestor = dirname(ancestor)
    add(ancestor.replaceAll('\\', '/'))
  }
}

export class ConfigRefresh {
  generation = 0
  refreshing = 0
  private tail = Promise.resolve()
  private failure: { readonly error: unknown } | undefined

  async run(action: () => Promise<void>): Promise<void> {
    this.generation++
    this.refreshing++
    const previous = this.tail
    const completed = Promise.withResolvers<void>()
    this.tail = completed.promise
    try {
      await previous
      await action()
      this.failure = undefined
    } catch (error) {
      this.failure = { error }
      throw error
    } finally {
      this.refreshing--
      this.generation++
      completed.resolve()
    }
  }

  async ready(): Promise<void> {
    while (this.refreshing) await this.tail
    this.assertReady()
  }

  wait(): Promise<void> | undefined {
    if (this.refreshing) return this.ready()
    this.assertReady()
  }

  assertReady(): void {
    if (this.failure) throw this.failure.error
  }
}
