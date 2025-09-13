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
  if (existsSync('./benchmark/next-devup-ui-single/.next'))
    rmSync('./benchmark/next-devup-ui-single/.next', {
      recursive: true,
      force: true,
    })
  if (existsSync('./benchmark/next-mui/.next'))
    rmSync('./benchmark/next-mui/.next', {
      recursive: true,
      force: true,
    })
  if (existsSync('./benchmark/next-devup-ui/df'))
    rmSync('./benchmark/next-devup-ui/df', {
      recursive: true,
      force: true,
    })
  if (existsSync('./benchmark/next-devup-ui-single/df'))
    rmSync('./benchmark/next-devup-ui-single/df', {
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

performance.mark('kuma-ui-start')
console.profile('kuma-ui')
execSync('pnpm -F next-kuma-ui-benchmark build', {
  stdio: 'inherit',
})
console.profileEnd('kuma-ui')
performance.mark('kuma-ui-end')
performance.measure('kuma-ui', 'kuma-ui-start', 'kuma-ui-end')

performance.mark('chakra-ui-start')
console.profile('chakra-ui')
execSync('pnpm -F next-chakra-ui-benchmark build', {
  stdio: 'inherit',
})
console.profileEnd('chakra-ui')
performance.mark('chakra-ui-end')
performance.measure('chakra-ui', 'chakra-ui-start', 'chakra-ui-end')

performance.mark('mui-start')
console.profile('mui')
execSync('pnpm -F next-mui-benchmark build', {
  stdio: 'inherit',
})
console.profileEnd('mui')
performance.mark('mui-end')
performance.measure('mui', 'mui-start', 'mui-end')

performance.mark('devup-ui-start')
console.profile('devup-ui')
execSync('pnpm -F next-devup-ui-benchmark build', {
  stdio: 'inherit',
})
console.profileEnd('devup-ui')
performance.mark('devup-ui-end')
performance.measure('devup-ui', 'devup-ui-start', 'devup-ui-end')

performance.mark('devup-ui-single-start')
console.profile('devup-ui-single')
execSync('pnpm -F next-devup-ui-single-benchmark build', {
  stdio: 'inherit',
})
console.profileEnd('devup-ui-single')
performance.mark('devup-ui-single-end')
performance.measure(
  'devup-ui-single',
  'devup-ui-single-start',
  'devup-ui-single-end',
)

console.info(performance.getEntriesByName('kuma-ui'))

console.info(
  'kuma-ui',
  checkDirSize('./benchmark/next-kuma-ui/.next').toLocaleString() + 'bytes',
)

console.info(performance.getEntriesByName('chakra-ui'))

console.info(
  'chakra-ui',
  checkDirSize('./benchmark/next-chakra-ui/.next').toLocaleString() + 'bytes',
)

console.info(performance.getEntriesByName('mui'))

console.info(
  'mui',
  checkDirSize('./benchmark/next-mui/.next').toLocaleString() + 'bytes',
)

console.info(performance.getEntriesByName('devup-ui'))

console.info(
  'devup-ui',
  checkDirSize('./benchmark/next-devup-ui/.next').toLocaleString() + 'bytes',
)

console.info(performance.getEntriesByName('devup-ui-single'))

console.info(
  'devup-ui-single',
  checkDirSize('./benchmark/next-devup-ui-single/.next').toLocaleString() +
    'bytes',
)
