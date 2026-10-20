import { readFile } from 'node:fs/promises'

export async function readPrivateBytes(pid, read = readFile) {
  try {
    const contents = await read(`/proc/${pid}/smaps_rollup`, 'utf8')
    return [
      ...contents.matchAll(/^Private_(?:Clean|Dirty|Hugetlb):\s+(\d+) kB$/gm),
    ].reduce((total, match) => total + Number(match[1]) * 1024, 0)
  } catch (error) {
    if (
      error instanceof Error &&
      'code' in error &&
      (error.code === 'ENOENT' || error.code === 'ESRCH')
    )
      return 0
    throw error
  }
}
