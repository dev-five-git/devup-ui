import { setTimeout as delay } from 'node:timers/promises'

export function sameProcess(expected, current) {
  return (
    expected !== undefined &&
    current !== undefined &&
    expected.pid === current.pid &&
    expected.born !== undefined &&
    expected.born === current.born
  )
}

export class ProcessOwnership {
  owned = new Map()
  retired = new Set()
  closed = false

  constructor(runtime) {
    this.runtime = runtime
  }

  async establish(pid) {
    this.rootPid = pid
    const root = await this.runtime.inspect(pid)
    if (root) this.owned.set(pid, root)
  }

  async validate(row) {
    const current = await this.runtime.inspect(row.pid)
    if (sameProcess(row, current)) return current
    this.owned.delete(row.pid)
    this.retired.add(row.pid)
    return undefined
  }

  async discover(rows) {
    for (const row of this.owned.values()) {
      if (
        !sameProcess(
          row,
          rows.find((current) => current.pid === row.pid),
        )
      ) {
        this.owned.delete(row.pid)
        this.retired.add(row.pid)
      }
    }
    let added
    do {
      added = false
      for (const row of rows) {
        if (this.closed) return
        const parent = this.owned.get(row.parent)
        if (
          row.pid === this.rootPid ||
          this.retired.has(row.pid) ||
          this.owned.has(row.pid) ||
          !parent
        )
          continue
        if (!(await this.validate(parent))) continue
        const current = await this.runtime.inspect(row.pid)
        if (
          sameProcess(row, current) &&
          current.parent === parent.pid &&
          BigInt(current.bornOrder) >= BigInt(parent.bornOrder) &&
          (await this.validate(parent)) &&
          !this.closed
        ) {
          this.owned.set(row.pid, current)
          added = true
        }
      }
    } while (added)
  }

  async memory() {
    let bytes = 0
    for (const row of this.owned.values()) {
      if (this.closed) return 0
      const current = await this.validate(row)
      if (!current || this.closed) continue
      const value = await this.runtime.privateBytes(current)
      if ((await this.validate(row)) && !this.closed) bytes += value
    }
    return bytes
  }

  async stop(timeoutMs) {
    const errors = []
    for (const row of [...this.owned.values()].reverse()) {
      try {
        // Generation check is NOT atomic with the subsequent PID signal (no pidfd).
        if (!(await this.validate(row))) continue
      } catch (error) {
        errors.push(error)
        continue
      }
      try {
        await this.runtime.kill(row.pid)
      } catch (error) {
        if (
          error instanceof Error &&
          error.code === 'ESRCH' &&
          error.syscall === 'kill'
        ) {
          try {
            if (!(await this.validate(row))) continue
          } catch (inspectionError) {
            errors.push(inspectionError)
          }
        }
        errors.push(error)
      }
    }
    const deadline = performance.now() + timeoutMs
    while (this.owned.size) {
      for (const row of [...this.owned.values()]) {
        try {
          await this.validate(row)
        } catch (error) {
          errors.push(error)
          this.owned.delete(row.pid)
        }
      }
      if (!this.owned.size) break
      if (performance.now() >= deadline) {
        errors.push(
          new Error(`Owned process drain timed out: ${[...this.owned.keys()]}`),
        )
        break
      }
      await delay(20)
    }
    return errors
  }
}
