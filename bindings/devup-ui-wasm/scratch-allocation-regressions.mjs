import assert from 'node:assert/strict'

const PACKAGE = '@devup-ui/react'
const supplier = `import {style,createVar,keyframes} from '${PACKAGE}';
export const base=style({color:'salmon'});
export const token=createVar();
export const spin=keyframes({from:{opacity:0},to:{opacity:1}});`
const bad = `import {Box,keyframes} from '${PACKAGE}';
const spin=keyframes({from:{opacity:0},to:{opacity:1}});
export const view=<Box color='salmon' gap={value} animation={\`${'${spin}'} 1s\`}/>;
import {base} from './supplier.css'; import {FAULT} from './fault';
export const tail=<Box className={base} bg={FAULT}/>;`
const good = `import {style} from '${PACKAGE}';
import {base,token} from './supplier.css';
export const own=style([base,{background:token}]);`

function snapshot(wasm) {
  return {
    slots: JSON.parse(wasm.exportClassMap()),
    files: JSON.parse(wasm.exportFileMap()),
    sheet: JSON.parse(wasm.exportSheet()),
    reservation: JSON.parse(wasm.exportReservationState()),
    misses: wasm.getUnreservedClassSlotCount(),
  }
}

export function assertScratchAllocationRollback(wasm, method) {
  for (const singleCss of [false, true]) {
    for (const kind of ['throw', 'malformed']) {
      // Given: the control keeps the same FILE inventory, skipping only bad extraction.
      const rows = [
        ['/src/bad.tsx', bad, 'df'],
        ['/src/good.css.ts', good, 'df'],
      ]
      const reserve = (inputs) =>
        wasm.reserveClassSlots(inputs, PACKAGE, singleCss, false, false, {})
      const extract = (row) =>
        wasm[method](row[0], row[1], PACKAGE, 'df', singleCss, false, false, {})
      let prepared = false
      let partial = 0
      const resolver = (specifier) => {
        if (specifier === './fault') {
          partial++
          if (!prepared) {
            if (kind === 'throw') throw new Error('late host fault')
            return { path: '/src/fault.ts', code: null }
          }
          return { path: '/src/fault.ts', code: "export const FAULT='orchid';" }
        }
        assert.equal(specifier, './supplier.css')
        return { path: '/src/supplier.css.ts', code: supplier }
      }
      const reset = () => {
        wasm.resetBuildState()
        wasm.setDebug(false)
        wasm.setModuleResolver(resolver)
        wasm.seedFileMap([
          '/src/bad.tsx',
          '/src/good.css.ts',
          '/src/supplier.css.ts',
          '/src/fault.ts',
        ])
      }
      reset()
      assert.throws(
        () => extract(rows[0]),
        /module resolver cannot use `\.\/fault`/,
      )
      assert.ok(
        Object.values(snapshot(wasm).slots).reduce(
          (sum, entries) => sum + Object.keys(entries).length,
          0,
        ) >= 4,
        'the real faulting extraction allocates before its result is rejected',
      )
      reset()
      partial = 0
      reserve([[rows[0][0], '', 'df'], rows[1]])
      const control = snapshot(wasm)
      assert.equal(wasm.getSuppressedReservationRowCount(), 0)
      reset()
      // When
      reserve(rows)
      // Then
      assert.equal(partial, 1)
      const failed = snapshot(wasm)
      assert.deepEqual(failed.slots, control.slots)
      assert.deepEqual(failed.files, control.files)
      assert.deepEqual(failed.sheet, control.sheet)
      assert.deepEqual(failed.reservation, {
        ...control.reservation,
        suppressed: 1,
      })
      assert.equal(failed.misses, control.misses)
      assert.equal(wasm.getSuppressedReservationRowCount(), 1)
      assert.equal(wasm.getUnreservedClassSlotCount(), 0)

      // Given / When / Then: an owner reset/restore carries suppression separately.
      reset()
      assert.equal(wasm.getSuppressedReservationRowCount(), 0)
      wasm.importSheet(failed.sheet)
      wasm.importClassMap(failed.slots)
      wasm.importFileMap(failed.files)
      wasm.importReservationState(failed.reservation)
      assert.equal(wasm.getSuppressedReservationRowCount(), 1)
      assert.equal(wasm.getUnreservedClassSlotCount(), 0)

      // Given / When / Then: preparation on the next batch appends, never renumbers.
      prepared = true
      reserve(rows)
      assert.equal(wasm.getSuppressedReservationRowCount(), 0)
      const retried = snapshot(wasm)
      assert.deepEqual(retried.files, failed.files)
      assert.deepEqual(retried.sheet, failed.sheet)
      for (const [file, entries] of Object.entries(failed.slots)) {
        for (const [key, slot] of Object.entries(entries)) {
          assert.equal(retried.slots[file][key], slot)
        }
      }
      const count = (slots) =>
        Object.values(slots).reduce(
          (sum, entries) => sum + Object.keys(entries).length,
          0,
        )
      assert.ok(count(retried.slots) > count(failed.slots))
      const output = extract(rows[0])
      assert.doesNotMatch(output.code, /<Box/)
      assert.match(output.css ?? wasm.getCss(undefined, false), /orchid/)
      output.free()
      assert.deepEqual(snapshot(wasm).slots, retried.slots)
      assert.equal(wasm.getUnreservedClassSlotCount(), 0)
      reserve([])
      assert.equal(wasm.getSuppressedReservationRowCount(), 0)
    }
  }
  wasm.setModuleResolver(undefined)
  wasm.resetBuildState()
}
