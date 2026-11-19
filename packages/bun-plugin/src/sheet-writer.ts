import { writeFileSync } from 'node:fs'

/**
 * The runtime stylesheet writer: `schedule()` asks for the sheet `read()`
 * gives to be written to `path()`. Every module that adds styles schedules a
 * write, and the modules loaded in one synchronous stretch share a single
 * write of the latest sheet, made in the microtask after them, instead of each
 * rewriting the whole sheet while it loads.
 *
 * The write stays synchronous, and runs before the code that awaited the
 * import resumes, so the sheet on disk is up to date as soon as an `await
 * import()` settles, as it was when every module wrote its own: tests and
 * tools read it right then. A deferred asynchronous write would leave them a
 * stale sheet.
 */
export function createSheetWriter(path: () => string, read: () => string) {
  let due = false
  function write() {
    due = false
    try {
      writeFileSync(path(), read(), 'utf-8')
    } catch (error) {
      // A microtask has no caller to throw to
      console.error(error)
    }
  }
  return function schedule() {
    if (due) return
    due = true
    queueMicrotask(write)
  }
}
