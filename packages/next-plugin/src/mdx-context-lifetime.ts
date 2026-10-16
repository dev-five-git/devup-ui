export type MdxLoaderOccurrence = {
  readonly loader: string
  readonly index: number
  readonly phase: 'normal' | 'pitch'
}

export class MdxContextRefusal extends Error {
  readonly name = 'MdxContextRefusal'
  readonly loader: string
  readonly index: number
  readonly phase: MdxLoaderOccurrence['phase']
  constructor(
    occurrence: MdxLoaderOccurrence,
    readonly property: string,
    readonly reason: string,
  ) {
    super(
      `loader ${occurrence.loader} occurrence ${occurrence.index} ${occurrence.phase} cannot use ${property}: ${reason}; use source/options/dependency APIs during their active occurrence or the receipted native compilation`,
    )
    this.loader = occurrence.loader
    this.index = occurrence.index
    this.phase = occurrence.phase
  }
}

type Lease = {
  readonly context: object
  readonly occurrence: MdxLoaderOccurrence
  open: boolean
  deferred: boolean
}

export function createMdxContextLifetime() {
  let first: MdxContextRefusal | undefined
  let terminal = false
  let current: Lease | undefined
  const refuse = (
    occurrence: MdxLoaderOccurrence,
    property: PropertyKey,
    reason: string,
  ): never => {
    first ??= new MdxContextRefusal(occurrence, String(property), reason)
    throw first
  }
  return {
    check() {
      if (first) throw first
    },
    close() {
      terminal = true
      if (current) current.open = false
      return first
    },
    advance() {
      if (current) current.open = false
    },
    run(
      context: object,
      occurrence: MdxLoaderOccurrence,
      execute: () => unknown,
    ) {
      if (terminal)
        return refuse(occurrence, 'context', 'invocation has completed')
      if (current) current.open = false
      const lease: Lease = { context, occurrence, open: true, deferred: false }
      current = lease
      try {
        const value = execute()
        if (!lease.deferred && (value === null || typeof value !== 'object'))
          lease.open = false
        return value
      } catch (cause) {
        lease.open = false
        throw cause
      }
    },
    access(context: object, occurrence: MdxLoaderOccurrence) {
      const lease = current
      if (
        !lease ||
        lease.context !== context ||
        lease.occurrence !== occurrence
      )
        return refuse(
          occurrence,
          'context',
          'original runner occurrence is unavailable',
        )
      const guard = (property: PropertyKey) => {
        if (terminal || !lease.open || current !== lease)
          return refuse(
            occurrence,
            property,
            'original runner occurrence has completed',
          )
      }
      guard('context')
      return {
        guard,
        refuse: (property: PropertyKey) =>
          refuse(
            occurrence,
            property,
            'outside the finite preparation context',
          ),
        terminal: () => terminal,
        defer: () => {
          lease.deferred = true
        },
        complete: () => {
          lease.open = false
        },
      }
    },
  }
}
