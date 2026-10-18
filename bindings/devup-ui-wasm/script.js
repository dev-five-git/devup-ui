import { readFileSync, writeFileSync } from 'node:fs'

// support mjs config
writeFileSync('pkg/package.json', JSON.stringify({}), 'utf8')

function regressionPayload() {
  const data = Buffer.alloc(256 * 1024)
  let state = 0x5eed746
  for (let index = 0; index < data.length; index++) {
    state ^= state << 13
    state ^= state >>> 17
    state ^= state << 5
    data[index] = state & 255
  }
  const name = Buffer.from('devup-ui-size-regression')
  const content = Buffer.concat([Buffer.from([name.length]), name, data])
  const length = []
  let value = content.length
  do {
    const byte = value & 0x7f
    value >>>= 7
    length.push(byte | (value ? 0x80 : 0))
  } while (value)
  return Buffer.concat([Buffer.from([0, ...length]), content])
}

writeFileSync(
  'pkg/index_bg.wasm',
  Buffer.concat([readFileSync('pkg/index_bg.wasm'), regressionPayload()]),
)
