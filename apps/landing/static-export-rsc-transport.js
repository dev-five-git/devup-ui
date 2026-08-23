/**
 * Adapt vinext's RSC requests to the `.rsc` artifacts emitted by
 * `output: 'export'`. Plain static hosts cannot vary a clean route URL by the
 * `RSC` request header, so the browser fetches the matching artifact directly
 * and restores the response metadata expected by vinext's navigation runtime.
 */
export function installStaticExportRscTransport(deploymentId) {
  const nativeFetch = globalThis.fetch.bind(globalThis)

  // Vinext's static artifacts contain full route payloads. Automatically
  // prefetching every visible link would decode those payloads and eagerly load
  // their route assets on the landing page, so keep them demand-driven.
  Object.defineProperty(globalThis, '__VINEXT_LINK_PREFETCH_ROUTES__', {
    configurable: true,
    get: () => undefined,
    set: () => {},
  })

  globalThis.fetch = async (input, init) => {
    const request = new Request(input, init)

    if (request.method !== 'GET' || request.headers.get('RSC') !== '1') {
      return nativeFetch(input, init)
    }

    const visibleUrl = new URL(request.url)
    if (visibleUrl.origin !== globalThis.location.origin) {
      return nativeFetch(input, init)
    }

    const artifactUrl = new URL(visibleUrl)
    if (!artifactUrl.pathname.endsWith('.rsc')) {
      artifactUrl.pathname =
        artifactUrl.pathname === '/'
          ? '/index.rsc'
          : `${artifactUrl.pathname.replace(/\/$/u, '')}.rsc`
    }
    artifactUrl.searchParams.delete('_rsc')

    const artifactResponse = await nativeFetch(artifactUrl, {
      credentials: request.credentials,
      headers: request.headers,
      signal: request.signal,
    })

    if (!artifactResponse.ok) {
      await artifactResponse.body?.cancel()
      return nativeFetch(input, init)
    }

    const headers = new Headers(artifactResponse.headers)
    headers.set('Content-Type', 'text/x-component')
    headers.set('X-Vinext-RSC-Compatibility-Id', deploymentId)

    const response = new Response(artifactResponse.body, {
      headers,
      status: artifactResponse.status,
      statusText: artifactResponse.statusText,
    })

    // The transport URL is an implementation detail, not a redirect target.
    Object.defineProperty(response, 'url', { value: visibleUrl.href })
    return response
  }
}
