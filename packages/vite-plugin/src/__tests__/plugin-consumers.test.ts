import { join } from 'node:path'

import { expect, it, mock } from 'bun:test'

import { environment, lifecycle, watcher } from './plugin-lifecycle-fixture'

it.each(['hotUpdate', 'handleHotUpdate'] as const)(
  'hard-invalidates live compiled consumers and tolerates missing modules through %s',
  async (hook) => {
    // Given: real extraction supplies edges; framework fakes only record invalidations.
    using state = await lifecycle()
    const { project, plugin } = state
    project.write('src/tone.ts', 'export const tone="red"')
    const supplier = join(project.root, 'src/tone.ts')
    const client = join(project.root, 'src/client.tsx')
    const server = join(project.root, 'src/server.tsx')
    const missing = join(project.root, 'src/missing.tsx')
    const code =
      'import {Box} from "@devup-ui/react"; import {tone} from "./tone"; export const A=()=> <Box color={tone}/>'
    const addWatchFile = mock()
    for (const [name, id] of [
      ['client', client],
      ['ssr', server],
      ['client', missing],
    ]) {
      project.write(`src/${name}.tsx`, code)
      await plugin.transform.call(
        { environment: { name }, addWatchFile },
        code,
        id,
      )
    }
    expect(addWatchFile).toHaveBeenCalledWith(supplier.replaceAll('\\', '/'))
    const current = environment('client', [client])
    const event = { file: supplier, modules: [], timestamp: 17 }
    // When
    switch (hook) {
      case 'hotUpdate':
        await plugin.hotUpdate.call({ environment: current }, event)
        break
      case 'handleHotUpdate':
        await plugin.handleHotUpdate({
          ...event,
          server: { moduleGraph: current.moduleGraph, ws: current.hot },
        })
        break
      default:
        throw new TypeError(`Unexpected hook ${hook satisfies never}`)
    }
    // Then
    expect(current.moduleGraph.invalidateModule).toHaveBeenCalledTimes(1)
    expect(current.moduleGraph.invalidateModule).toHaveBeenCalledWith(
      current.moduleGraph.getModuleById(client),
      expect.any(Set),
      17,
      true,
    )
    expect(current.hot.send).not.toHaveBeenCalled()
  },
)

it('replaces consumer edges per environment and removes all query variants on actual watcher unlink', async () => {
  // Given
  using state = await lifecycle()
  const { project, plugin } = state
  project.write('src/tone.ts', 'export const tone="blue"')
  const supplier = join(project.root, 'src/tone.ts')
  const id = join(project.root, 'src/App.tsx')
  const code =
    'import {Box} from "@devup-ui/react"; import {tone} from "./tone"; export const A=()=> <Box color={tone}/>'
  project.write('src/App.tsx', code)
  const watching = watcher()
  plugin.configureServer({ config: state.config, watcher: watching })
  for (const name of ['client', 'ssr'])
    await plugin.transform.call(
      { environment: { name }, addWatchFile: mock() },
      code,
      `${id}?${name}`,
    )
  await plugin.transform.call(
    { environment: { name: 'client' }, addWatchFile: mock() },
    'export const A=1',
    `${id}?client`,
  )
  const client = environment('client', [`${id}?client`])
  const ssr = environment('ssr', [`${id}?ssr`])
  const event = { file: supplier, modules: [], timestamp: 21 }
  await plugin.hotUpdate.call({ environment: client }, event)
  await plugin.hotUpdate.call({ environment: ssr }, event)
  expect(client.moduleGraph.invalidateModule).not.toHaveBeenCalled()
  expect(ssr.moduleGraph.invalidateModule).toHaveBeenCalledTimes(1)
  ssr.moduleGraph.invalidateModule.mockClear()
  // When
  watching.emit('all', 'unlink', id)
  await plugin.hotUpdate.call({ environment: ssr }, { ...event, timestamp: 22 })
  // Then
  expect(ssr.moduleGraph.invalidateModule).not.toHaveBeenCalled()
})
