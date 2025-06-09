/* eslint-disable no-console */
import { existsSync, readdirSync, rmSync, statSync } from 'node:fs'
import { join } from 'node:path'

import { execSync } from 'child_process'

function clearBuildFile() {
  if (existsSync('./benchmark/next-kuma-ui/.next'))
    rmSync('./benchmark/next-kuma-ui/.next', {
      recursive: true,
      force: true,
    })
  if (existsSync('./benchmark/next-chakra-ui/.next'))
    rmSync('./benchmark/next-chakra-ui/.next', {
      recursive: true,
      force: true,
    })
  if (existsSync('./benchmark/next-devup-ui/.next'))
    rmSync('./benchmark/next-devup-ui/.next', {
      recursive: true,
      force: true,
    })
  if (existsSync('./benchmark/next-devup-ui/.df'))
    rmSync('./benchmark/next-devup-ui/.df', {
      recursive: true,
      force: true,
    })
}

function checkDirSize(path) {
  let totalSize = 0

  function calculateSize(directory) {
    const entries = readdirSync(directory)
    for (const entry of entries) {
      const entryPath = join(directory, entry)
      if (statSync(entryPath).isDirectory()) {
        calculateSize(entryPath) // 재귀적으로 하위 폴더 크기 계산
      } else {
        const stats = statSync(entryPath)
        totalSize += stats.size // 파일 크기 합산
      }
    }
  }

  calculateSize(path)
  return totalSize
}

clearBuildFile()

console.time('kuma-ui')
execSync('pnpm -F next-kuma-ui-benchmark build', {
  stdio: 'inherit',
})
console.timeEnd('kuma-ui')
console.info('kuma-ui', checkDirSize('./benchmark/next-kuma-ui/.next'))

console.time('chakra-ui')
execSync('pnpm -F next-chakra-ui-benchmark build', {
  stdio: 'inherit',
})
console.timeEnd('chakra-ui')
console.info('chakra-ui', checkDirSize('./benchmark/next-chakra-ui/.next'))

console.time('devup-ui')
execSync('pnpm -F next-devup-ui-benchmark build', {
  stdio: 'inherit',
})
console.timeEnd('devup-ui')
console.info('devup-ui', checkDirSize('./benchmark/next-devup-ui/.next'))
