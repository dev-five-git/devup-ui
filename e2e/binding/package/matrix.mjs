import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'

// Canonical build target table. GNU cross builds use a current Zig with glibc 2.28.
export const targets = Object.freeze(
  Object.entries({
    'linux-x64-gnu': 'x86_64-unknown-linux-gnu',
    'linux-arm64-gnu': 'aarch64-unknown-linux-gnu',
    'linux-x64-musl': 'x86_64-unknown-linux-musl',
    'linux-arm64-musl': 'aarch64-unknown-linux-musl',
    'darwin-x64': 'x86_64-apple-darwin',
    'darwin-arm64': 'aarch64-apple-darwin',
    'win32-x64-msvc': 'x86_64-pc-windows-msvc',
    'win32-arm64-msvc': 'aarch64-pc-windows-msvc',
  }).map(([suffix, target]) => {
    const [os, cpu, abi] = suffix.split('-')
    return Object.freeze({
      suffix,
      target,
      os,
      cpu,
      ...(os === 'linux' ? { libc: abi === 'gnu' ? 'glibc' : 'musl' } : {}),
    })
  }),
)

// Decode actual target-machine bytes and dynamic imports, not filename guesses.
function region(bytes, offset, size) {
  assert.ok(
    Number.isSafeInteger(offset) &&
      Number.isSafeInteger(size) &&
      offset >= 0 &&
      size >= 0 &&
      offset <= bytes.length - size,
    'invalid binary region',
  )
  return bytes.subarray(offset, offset + size)
}
function stringAt(bytes, offset) {
  region(bytes, offset, 1)
  const end = bytes.indexOf(0, offset)
  assert.ok(end > offset, 'missing or empty binary string')
  return bytes.toString('utf8', offset, end)
}
function peEvidence(bytes) {
  const pe = bytes.readUInt32LE(0x3c)
  assert.equal(bytes.toString('hex', pe, pe + 4), '50450000')
  const machine = bytes.readUInt16LE(pe + 4)
  const optional = pe + 24
  assert.equal(bytes.readUInt16LE(optional), 0x20b)
  const sections = []
  const table = optional + bytes.readUInt16LE(pe + 20)
  for (let index = 0; index < bytes.readUInt16LE(pe + 6); index++) {
    const offset = table + index * 40
    sections.push({
      size: Math.max(
        bytes.readUInt32LE(offset + 8),
        bytes.readUInt32LE(offset + 16),
      ),
      rva: bytes.readUInt32LE(offset + 12),
      raw: bytes.readUInt32LE(offset + 20),
    })
  }
  const offsetOf = (rva) => {
    const section = sections.find(
      (item) => rva >= item.rva && rva < item.rva + item.size,
    )
    assert.ok(section, 'PE RVA must belong to a section')
    return rva - section.rva + section.raw
  }
  const dependencies = []
  const importRva = bytes.readUInt32LE(optional + 120)
  assert.ok(importRva, 'PE must contain imports')
  for (
    let offset = offsetOf(importRva);
    bytes.readUInt32LE(offset + 12);
    offset += 20
  ) {
    const name = offsetOf(bytes.readUInt32LE(offset + 12))
    dependencies.push(stringAt(bytes, name))
  }
  return {
    format: 'PE32+',
    machine,
    cpu:
      machine === 0x8664 ? 'x64' : machine === 0xaa64 ? 'arm64' : 'unsupported',
    dependencies: dependencies.sort(),
  }
}
function elfEvidence(bytes, row) {
  const header = region(bytes, 0, 64)
  assert.equal(header.toString('hex', 0, 6), '7f454c460201')
  const machine = header.readUInt16LE(18)
  const stride = header.readUInt16LE(54)
  assert.ok(stride >= 56, 'invalid ELF program header size')
  const table = region(
    bytes,
    Number(header.readBigUInt64LE(32)),
    stride * header.readUInt16LE(56),
  )
  const loads = []
  let dynamic
  for (let offset = 0; offset < table.length; offset += stride) {
    const type = table.readUInt32LE(offset)
    if (type !== 1 && type !== 2) continue
    const file = Number(table.readBigUInt64LE(offset + 8))
    const address = table.readBigUInt64LE(offset + 16)
    const size = Number(table.readBigUInt64LE(offset + 32))
    const data = region(bytes, file, size)
    if (type === 1) loads.push({ address, data })
    else {
      assert.equal(dynamic, undefined, 'duplicate ELF dynamic segment')
      dynamic = data
    }
  }
  assert.ok(
    dynamic?.length && dynamic.length % 16 === 0,
    'missing or invalid ELF dynamic records',
  )
  const records = new Map()
  let terminated = false
  for (let offset = 0; offset < dynamic.length; offset += 16) {
    const tag = dynamic.readBigUInt64LE(offset)
    if (tag === 0n) {
      terminated = true
      break
    }
    records.set(tag, [
      ...(records.get(tag) || []),
      dynamic.readBigUInt64LE(offset + 8),
    ])
  }
  assert.ok(terminated, 'unterminated ELF dynamic records')
  assert.equal(
    records.get(5n)?.length,
    1,
    'missing or duplicate ELF string table',
  )
  assert.equal(
    records.get(10n)?.length,
    1,
    'missing or duplicate ELF string size',
  )
  const address = records.get(5n)[0]
  const size = records.get(10n)[0]
  assert.ok(size > 0n, 'empty ELF string table')
  const load = loads.find(
    (item) =>
      address >= item.address &&
      address + size <= item.address + BigInt(item.data.length),
  )
  assert.ok(load, 'ELF string table must be file-backed')
  const strings = region(
    load.data,
    Number(address - load.address),
    Number(size),
  )
  const dependencies = (records.get(1n) || [])
    .map((value) => stringAt(strings, Number(value)))
    .sort()
  const musl = dependencies.some((name) => name.startsWith('libc.musl-'))
  const expectedMusl = dependencies.includes(
    `libc.musl-${row.cpu === 'x64' ? 'x86_64' : 'aarch64'}.so.1`,
  )
  const glibc = dependencies.includes('libc.so.6')
  assert.ok(
    row.libc === 'musl'
      ? expectedMusl && !glibc
      : row.libc === 'glibc' && glibc && !musl,
    'wrong libc',
  )
  return {
    format: 'ELF64',
    machine,
    cpu: machine === 62 ? 'x64' : machine === 183 ? 'arm64' : 'unsupported',
    dependencies,
  }
}
function machoEvidence(bytes) {
  const header = region(bytes, 0, 32)
  assert.equal(header.readUInt32LE(0), 0xfeedfacf)
  const machine = header.readUInt32LE(4)
  const commands = region(bytes, 32, header.readUInt32LE(20))
  const dependencies = []
  let offset = 0
  for (let index = 0; index < header.readUInt32LE(16); index++) {
    const command = region(commands, offset, 8)
    const size = command.readUInt32LE(4)
    assert.ok(size >= 8 && size % 8 === 0, 'invalid Mach-O command size')
    const data = region(commands, offset, size)
    // LOAD, WEAK, REEXPORT, LAZY and UPWARD dylibs; never LC_ID_DYLIB.
    if (
      [0xc, 0x80000018, 0x8000001f, 0x20, 0x80000023].includes(
        command.readUInt32LE(0),
      )
    ) {
      region(data, 0, 24)
      const name = data.readUInt32LE(8)
      assert.ok(name >= 24, 'invalid Mach-O dylib name offset')
      dependencies.push(stringAt(data, name))
    }
    offset += size
  }
  assert.equal(offset, commands.length, 'Mach-O command count/size mismatch')
  return {
    format: 'Mach-O64',
    machine,
    cpu:
      machine === 0x1000007
        ? 'x64'
        : machine === 0x100000c
          ? 'arm64'
          : 'unsupported',
    dependencies: dependencies.sort(),
  }
}
export function inspectImage(path, row) {
  const bytes = readFileSync(path)
  let evidence
  switch (row.os) {
    case 'win32':
      evidence = peEvidence(bytes)
      break
    case 'linux':
      evidence = elfEvidence(bytes, row)
      break
    case 'darwin':
      evidence = machoEvidence(bytes)
      break
    default:
      assert.fail('unsupported image format')
  }
  assert.equal(evidence.cpu, row.cpu, 'wrong image machine')
  assert.ok(
    evidence.dependencies.length,
    'dynamic dependencies must be nonempty',
  )
  return evidence
}
