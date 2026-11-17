/**
 * Keep vinext's link prefetching demand-driven on the static export.
 *
 * vinext reads the routes it may prefetch from
 * `__VINEXT_LINK_PREFETCH_ROUTES__`, and under `output: 'export'` every match
 * is a full prefetch of the route's exported `.txt` payload. Prefetching every
 * visible docs link would decode those payloads and eagerly load their route
 * assets, so hide the manifest from the prefetcher. Client navigation still
 * fetches the exported payload of the page it opens.
 */
export function disableLinkAutoPrefetch(): void {
  Object.defineProperty(globalThis, '__VINEXT_LINK_PREFETCH_ROUTES__', {
    configurable: true,
    get: () => undefined,
    set: () => {},
  })
}
