import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { rmSync, writeFileSync } from 'node:fs'
import { resolve } from 'node:path'

import { observeRawControlBuild } from './mdx-raw-controls-build.mjs'
import { createRawControlFixture } from './mdx-raw-controls-fixture.mjs'

const singleCss = process.argv[2] === 'true'
const fixture = createRawControlFixture()
const { root, page, description, widget } = fixture
const results = []
const positives = new Map()
const refusals = new Map()
const digest = (value) =>
  createHash('sha256').update(JSON.stringify(value)).digest('hex')
try {
  for (const kind of [
    'ordinary',
    'raw-alias',
    'raw-alias-operative',
    'external-unresolved',
    'external-resolved',
  ]) {
    for (const repeat of [1, 2]) {
      // Given cold identical sources per policy, including independently compiled MDX.
      const request =
        kind === 'raw-alias-operative' ? './Missing.mdx' : './Description.mdx'
      writeFileSync(
        page,
        `import {Text} from '@devup-ui/react'; import Doc from '${request}'; import './other.js'; import './widget.js'; console.info(<Text color='red'><Doc/></Text>);`,
      )
      rmSync(`${root}/df`, { recursive: true, force: true })
      const {
        observed,
        externalCalls,
        stages,
        graph,
        obligations,
        optionsSeen,
        failure,
        output,
        alias,
      } = await observeRawControlBuild(fixture, { kind, request, singleCss })
      const details =
        failure instanceof Error
          ? `${failure.message}\n${JSON.stringify(failure.errors)}`
          : ''
      const located = /page\.tsx:\d+:\d+:.*deferred default cannot use/.test(
        details,
      )
      const result = {
        kind,
        repeat,
        singleCss,
        request,
        optionsSeen,
        stages,
        graph,
        obligations,
        observed,
        externalCalls,
        outputSha256: output && digest(output),
        graphSha256: digest(graph),
        accepted: failure === undefined,
        located,
        details,
      }
      results.push(result)
      console.info(
        JSON.stringify({ event: 'actual-raw-native-control', ...result }),
      )
      // Then a configured-but-inert alias preserves the exact native output/graph.
      assert(optionsSeen)
      assert(stages.unrelated > 0)
      assert(
        obligations.some(
          (obligation) =>
            obligation.request === request &&
            obligation.exports.includes('default'),
        ),
      )
      const pageGraph = graph.find((info) => info.id === page)
      assert(pageGraph)
      if (kind === 'ordinary' || kind === 'raw-alias') {
        assert.equal(failure, undefined, details)
        assert(output)
        const js = output
          .filter((item) => item.type === 'chunk')
          .map((item) => item.code)
          .join('\n')
        const css = output
          .filter((item) => item.type === 'asset')
          .map((item) => String(item.source))
          .join('\n')
        assert(!js.includes('Cannot run on the runtime'))
        assert(css.includes('color:red'))
        assert(js.includes('Actual current MDX'))
        assert(pageGraph.importedIds.includes(description))
        assert(stages.description > 0)
        const baseline = positives.get('ordinary')
        if (baseline) assert.deepEqual({ output, graph }, baseline)
        else positives.set('ordinary', { output, graph })
      } else {
        assert(failure && located, details)
        assert.equal(output, undefined)
        assert(
          graph.some(
            (info) => info.id === widget && info.parsed && !info.codeNull,
          ),
        )
        if (alias) {
          assert(pageGraph.importedIds.includes(widget))
          assert(pageGraph.importedIds.includes(resolve(widget)))
          assert(!pageGraph.importedIds.includes(description))
          assert(stages.description > 0)
        } else {
          assert(
            externalCalls.some(
              (call) =>
                call.selected &&
                call.isResolved === (kind === 'external-resolved'),
            ),
          )
          const target = graph.find(
            (info) =>
              info.id.replaceAll('\\', '/') === description &&
              info.codeNull &&
              !info.parsed,
          )
          assert(target)
          assert(pageGraph.importedIds.includes(target.id))
          assert.equal(stages.description, kind === 'external-resolved' ? 0 : 1)
        }
        const signature = {
          graph,
          stages,
          obligations,
          reason: failure.errors?.map((error) => error.reason),
          located,
        }
        if (repeat === 1) refusals.set(kind, signature)
        else assert.deepEqual(signature, refusals.get(kind))
      }
    }
  }
  console.info(
    JSON.stringify({
      event: 'raw-native-qualified',
      singleCss,
      positives: results.filter((result) => result.accepted).length,
      locatedRefusals: results.filter((result) => result.located).length,
      exactPositiveIdentity: true,
      repeatedRefusalIdentity: true,
    }),
  )
} finally {
  rmSync(root, { recursive: true, force: true })
}
