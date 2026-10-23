const assert = require('node:assert/strict')

async function main() {
  // Given
  const { beginBuild, getBuildOwnership } = await import('../build-session.ts')
  assert.equal(getBuildOwnership().running, 0)
  let resets = 0
  const engine = {
    resetBuildState() {
      resets++
    },
  }
  // When
  const first = beginBuild(engine)
  // Then
  assert.equal(resets, 1)
  const before = resets
  const second = beginBuild(engine)
  assert.equal(resets, before)
  second()
  second()
  first()
  beginBuild({})()
  const third = beginBuild(engine)
  assert.equal(resets, before + 1)
  third()
  assert.equal(getBuildOwnership().running, 0)
}
main().catch((error) => {
  console.error(error)
  process.exitCode = 1
})
