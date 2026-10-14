import { execFileSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'

import { expect, it } from 'bun:test'

const fixture = fileURLToPath(
  new URL(
    '../../../../bindings/devup-ui-wasm/tests/value-evaluation.mjs',
    import.meta.url,
  ),
)
const cases = [
  'extract_conditional_style_props-2',
  'extract_dynamic_logical_case-3',
  'extract_responsive_conditional_style_props-3',
  'extract_responsive_conditional_style_props-4',
  'extract_responsive_conditional_style_props-5',
  'member_expression_with_dynamic_values',
  'props_direct_hybrid_responsive_select',
  'props_direct_variable_array_responsive_select',
  'props_direct_variable_array_responsive_select-2',
  'props_direct_variable_object_responsive_select',
  'logical_or',
  'logical_and',
  'getter_member',
  'null_member',
  'index_side_effects',
] as const

it.each(
  cases.flatMap((name) =>
    ['ordinary', 'single'].map((mode) => [name, mode] as const),
  ),
)(
  'preserves authored evaluation when public WASM lowers %s (%s)',
  (name, mode) => {
    // Given / When: fresh Node isolates public WASM state per fixture and mode.
    const output = execFileSync('node', [fixture, name, mode], {
      encoding: 'utf8',
      timeout: 60_000,
      env: { ...process.env, VALUE_EVALUATION_QA: '0' },
    })
    // Then: native authored/compiled evaluation and selected CSS agree.
    expect(output).toBe('')
  },
)
