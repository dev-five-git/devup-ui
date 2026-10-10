import { retainSession } from '../lifecycle'
import { type AppContext, createSession } from '../session'
import type { SetupHandoff } from '../setup-handoff'
import { createWasm } from '../wasm'

export function makeSetupOwner(context: AppContext, result: SetupHandoff) {
  const session = createSession(context)
  return retainSession({
    session: {
      ...session,
      token: result.sessionToken,
      identity: { project: context.root, token: result.sessionToken },
    },
    coordinator: {
      ready: Promise.resolve(),
      close: () => {},
      drain: async () => {},
    },
    setup: { context, engine: createWasm(context.root), result },
  })
}
