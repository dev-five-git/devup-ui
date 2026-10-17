import { join } from 'node:path'

import { expect, it } from 'bun:test'

import { retainSession, type SessionOwner } from '../lifecycle'
import { MdxCompileError } from '../mdx-prepare'
import { createAppContext, createSession } from '../session'
import { adapterFixture } from './adapter-fixture'

it('preserves the located preparation cause when owner cleanup releases its active adapter', async () => {
  // Given: a real final-config registration and its owner share a pending preparation.
  const fixture = adapterFixture()
  const ready = Promise.withResolvers<SessionOwner>()
  const cause = new MdxCompileError(
    join(fixture.session.projectDir, 'page.mdx'),
    Object.assign(new SyntaxError('configured compiler failed'), {
      place: { start: { line: 4, column: 7 } },
    }),
  )
  const installed = fixture.install({}, async () => {
    const owner = await ready.promise
    owner.close(cause)
    throw cause
  })
  const context = createAppContext({}, {}, fixture.session.projectDir)
  const owner = retainSession({
    session: createSession(context),
    coordinator: {
      ready: Promise.resolve(),
      close() {},
      drain: () => Promise.resolve(),
    },
    releaseAdapter: (reason) => installed.release(reason),
  })
  ready.resolve(owner)
  try {
    // When: preparation fails and synchronously closes its owner before returning the cause.
    const operation = installed.adapter.modifyConfig(
      installed.config,
      fixture.context,
    )
    // Then: the adapter reports the original MDX location/cause, not session cancellation.
    await expect(operation).rejects.toMatchObject({
      stage: 'finalizer',
      location: fixture.session.configFile,
      cause,
    })
  } finally {
    owner.close()
    fixture.close()
  }
})
