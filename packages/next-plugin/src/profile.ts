type ProfileFields = Record<string, boolean | number | string | undefined>

export function isProfileEnabled(): boolean {
  return process.env.DEVUP_UI_PROFILE === '1'
}

export function reportProfile(phase: string, fields: ProfileFields = {}): void {
  if (!isProfileEnabled()) return

  console.info(
    `[devup-ui:profile] ${JSON.stringify({
      phase,
      ...fields,
    })}`,
  )
}

export function profileStart(): number | undefined {
  return isProfileEnabled() ? performance.now() : undefined
}

export function elapsedMs(start: number | undefined): number | undefined {
  if (start === undefined) return undefined

  return Number((performance.now() - start).toFixed(2))
}
