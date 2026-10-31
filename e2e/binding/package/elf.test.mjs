import assert from 'node:assert/strict'
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { test } from 'node:test'

import { inspectImage, targets } from './matrix.mjs'

// Decoder-only rejection controls, never foreign-built positive evidence.
function rejectedElf(dependencies, tags, cpu) {
  const strings = Buffer.from(`\0${dependencies.join('\0')}\0`)
  let nameOffset = 1n
  const records = dependencies.map((name) => {
    const record = [1n, nameOffset]
    nameOffset += BigInt(Buffer.byteLength(name) + 1)
    return record
  })
  records.push([5n, 512n], [10n, BigInt(strings.length)])
  for (const tag of tags) records.push([tag, 0n])
  records.push([0n, 0n])
  const bytes = Buffer.alloc(512 + strings.length)
  bytes.write('7f454c4602010100', 0, 'hex')
  bytes.writeUInt16LE(3, 16)
  bytes.writeUInt16LE(cpu === 'x64' ? 62 : 183, 18)
  bytes.writeUInt32LE(1, 20)
  bytes.writeBigUInt64LE(64n, 32)
  bytes.writeUInt16LE(64, 52)
  bytes.writeUInt16LE(56, 54)
  bytes.writeUInt16LE(2, 56)
  // One file-backed PT_LOAD plus a separate, terminated PT_DYNAMIC.
  bytes.writeUInt32LE(1, 64)
  bytes.writeBigUInt64LE(BigInt(bytes.length), 96)
  bytes.writeBigUInt64LE(BigInt(bytes.length), 104)
  bytes.writeUInt32LE(2, 120)
  bytes.writeBigUInt64LE(176n, 128)
  bytes.writeBigUInt64LE(176n, 136)
  bytes.writeBigUInt64LE(BigInt(records.length * 16), 152)
  bytes.writeBigUInt64LE(BigInt(records.length * 16), 160)
  records.forEach(([tag, value], index) => {
    bytes.writeBigUInt64LE(tag, 176 + index * 16)
    bytes.writeBigUInt64LE(value, 184 + index * 16)
  })
  strings.copy(bytes, 512)
  return bytes
}

for (const cpu of ['x64', 'arm64']) {
  const own = `libc.musl-${cpu === 'x64' ? 'x86_64' : 'aarch64'}.so.1`
  const other = `libc.musl-${cpu === 'x64' ? 'aarch64' : 'x86_64'}.so.1`
  for (const [name, dependencies, tags] of [
    ['GNU presented as musl', ['libc.so.6'], []],
    ['generic with DT_VERNEED presence', ['libc.so'], [0x6ffffffen]],
    ['generic with DT_VERNEEDNUM presence', ['libc.so'], [0x6fffffffn]],
    ['generic with conflicting musl marker', ['libc.so', other], []],
    ['conventional with conflicting musl marker', [own, other], []],
    ['missing libc', ['libm.so'], []],
    ['generic mixed with GNU', ['libc.so', 'libc.so.6'], []],
    ['conventional mixed with GNU', [own, 'libc.so.6'], []],
    ['all libc spellings mixed with GNU', [own, 'libc.so', 'libc.so.6'], []],
  ]) {
    test(`rejects ${name} when inspecting ${cpu} musl`, () => {
      // Given: valid decoder framing; zero tag values still mean tag presence.
      const directory = mkdtempSync(join(tmpdir(), 'native-elf-rejection-'))
      const path = join(directory, 'control.node')
      const row = targets.find(
        (item) => item.cpu === cpu && item.libc === 'musl',
      )
      writeFileSync(path, rejectedElf(dependencies, tags, cpu))
      try {
        // When / Then: require the libc policy failure, not a framing failure.
        assert.throws(
          () => inspectImage(path, row),
          (error) => {
            assert.ok(error instanceof assert.AssertionError)
            assert.ok(error.message.startsWith('wrong libc: '))
            const diagnostic = JSON.parse(
              error.message.slice('wrong libc: '.length),
            )
            assert.deepEqual(diagnostic, {
              dependencies: [...dependencies].sort(),
              verneed: tags.includes(0x6ffffffen),
              verneednum: tags.includes(0x6fffffffn),
            })
            return true
          },
        )
      } finally {
        rmSync(directory, { recursive: true, force: true })
      }
    })
  }
}
