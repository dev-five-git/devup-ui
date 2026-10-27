const path = Bun.main.replaceAll('\\', '/')
if (/\/packages\/(?:components|react)\//.test(path)) {
  const { register } = await import('./packages/bun-plugin/src/register')
  await register({ debug: true })
}

export {}
