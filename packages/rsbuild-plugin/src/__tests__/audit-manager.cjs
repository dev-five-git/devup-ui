const { writeFileSync, readFileSync } = require('node:fs')
const { join } = require('node:path')
/** @type {typeof import('@devup-ui/plugin-utils')} */
const utils = require('@devup-ui/plugin-utils')
const { wasm, batches, extracts, state } = require('./audit-engine.cjs')

/** @param {string} root */
function manager(root) {
  wasm.resetBuildState()
  wasm.setDebug(false)
  wasm.setModuleResolver(utils.createModuleResolver({ cwd: root }))
  const slots = utils.createSharedSlotReservation({
    reserveClassSlots: wasm.reserveClassSlots,
  })
  const discovery = {
    roots: ['src'],
    cwd: root,
    package: '@devup-ui/react',
    importAliases: {},
  }
  const flags = {
    singleCss: true,
    importMainCssInCode: false,
    importMainCssInCss: true,
    cssDir: () => './css',
  }
  const initialInventory = utils.collectReservationInputs(discovery)
  const first = slots.reserve(discovery, flags)
  const input = join(root, 'src/entry.js')
  const code = readFileSync(input, 'utf8')
  wasm.codeExtract(
    input,
    code,
    '@devup-ui/react',
    './css',
    true,
    false,
    true,
    {},
  )
  const before = state()
  changeTarget(root)
  const revisedInventory = utils.collectReservationInputs(discovery)
  const second = slots.reserve(discovery, flags)
  wasm.codeExtract(
    input,
    code,
    '@devup-ui/react',
    './css',
    true,
    false,
    true,
    {},
  )
  return {
    first,
    second,
    before,
    after: state(),
    sameInputs:
      JSON.stringify(initialInventory.inputs) ===
      JSON.stringify(revisedInventory.inputs),
    sameSources:
      initialInventory.sources.size === revisedInventory.sources.size &&
      [...initialInventory.sources].every(
        ([path, text]) => revisedInventory.sources.get(path) === text,
      ),
    batches,
    extracts,
  }
}

/** @param {string} root */
function changeTarget(root) {
  writeFileSync(
    join(root, 'node_modules/@devup-ui/tokens/package.json'),
    JSON.stringify({
      exports: {
        '.': './blue.js',
        './red': './red.js',
        './blue': './blue.js',
      },
    }),
  )
}

module.exports = { manager, changeTarget }
