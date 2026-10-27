import assert from 'node:assert/strict'

export function writeClientFixture(write) {
  write(
    'app/client.jsx',
    `'use client';import {Box} from '@devup-ui/react';export default function Client({tone}){return <Box as="article" bg="maroon" color={tone}>native-client-dynamic</Box>}`,
  )
}

export function assertClientIdentities(html, assets) {
  const article = /<article\b([^>]*)>native-client-dynamic<\/article>/.exec(
    html,
  )
  assert.ok(article, 'native client HTML missing')
  const classes = /class="([^"]+)"/.exec(article[1])
  const variable = /style="[^"]*(--[\w-]+):rebeccapurple/.exec(article[1])
  assert.ok(classes, 'native client classes missing')
  assert.ok(variable, 'native dynamic inline variable missing')
  const css = assets.map(({ content }) => content).join('\n')
  const backgrounds = [
    ...css.matchAll(/((?:\.[\w-]+\s*,\s*)*\.[\w-]+)\{background:maroon\}/g),
  ].flatMap((match) => [...match[1].matchAll(/\.([\w-]+)/g)])
  const colors = [...css.matchAll(/\.([\w-]+)\{color:var\((--[\w-]+)\)\}/g)]
  assert.equal(
    new Set(backgrounds.map((match) => match[1])).size,
    1,
    'duplicate static identities',
  )
  assert.equal(
    new Set(colors.map((match) => match[1])).size,
    1,
    'duplicate dynamic identities',
  )
  assert.equal(
    colors.length,
    1,
    'dynamic declaration must be delivered exactly once',
  )
  const selected = new Set(classes[1].split(/\s+/))
  assert.ok(selected.has(backgrounds[0][1]), 'selected static class missing')
  assert.ok(selected.has(colors[0][1]), 'selected dynamic class missing')
  assert.equal(
    colors[0][2],
    variable[1],
    'selected class must read the inline variable',
  )
}
