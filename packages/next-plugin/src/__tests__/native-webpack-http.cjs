const assert = require('node:assert/strict')
const http = require('node:http')

/** @param {string | URL} url @param {AbortSignal} signal @returns {Promise<string>} */
function request(url, signal) {
  return new Promise((resolve, reject) => {
    const req = http.get(
      url,
      {
        agent: false,
        signal: AbortSignal.any([signal, AbortSignal.timeout(90000)]),
      },
      (res) => {
        let text = ''
        res.setEncoding('utf8')
        res.on('data', (chunk) => {
          text += chunk
        })
        res.on('error', reject)
        res.on('aborted', () =>
          reject(new Error(`HTTP response aborted: ${url}`)),
        )
        res.on('end', () => {
          if (res.statusCode !== 200) {
            reject(new Error(`HTTP ${res.statusCode}: ${url}\n${text}`))
            return
          }
          resolve(text)
        })
      },
    )
    req.on('error', reject)
  })
}
/** @param {string} base @param {boolean} revised @param {AbortSignal} signal */
async function surface(base, revised, signal) {
  const html = await request(base, signal)
  assert(html.includes('id="server-proof"'))
  assert(html.includes('id="client-proof"'))
  assert(html.includes('cyan'))
  const links = [...html.matchAll(/<link\b[^>]*>/g)]
    .map((match) => match[0])
    .filter((tag) => /rel="stylesheet"/.test(tag))
    .flatMap((tag) =>
      [...tag.matchAll(/href="([^"]+)"/g)].map((match) => {
        assert(match[1])
        return match[1]
      }),
    )
  assert(links.length > 0, 'first SSR response must link CSS')
  const css = (await assets(links, base, signal)).join('\n')
  for (const declaration of [
    'background:maroon',
    'padding:13px',
    'background:navy',
    '@keyframes',
    'color:var(--',
  ])
    assert(css.includes(declaration), `first CSS missing ${declaration}`)
  for (const [, classes] of html.matchAll(/class="([^"]+)"/g)) {
    assert(classes)
    for (const name of classes.split(/\s+/))
      assert(css.includes(`.${name}`), `SSR class ${name} has no actual CSS`)
  }
  const scripts = [...html.matchAll(/<script\b[^>]*src="([^"]+)"/g)].map(
    (match) => {
      assert(match[1])
      return match[1]
    },
  )
  assert(scripts.length > 0)
  const js = (await assets(scripts, base, signal)).join('\n')
  assert(!js.includes('Cannot run on the runtime'))
  assert(!js.includes('@devup-ui/react/dist/components/Box'))
  assert(!html.includes('<Box'))
  if (revised) {
    assert(html.includes('id="new-a"'))
    assert(html.includes('id="new-b"'))
    assert(css.includes('background:orchid'))
    assert(css.includes('background:seagreen'))
  }
  return { html, css, js, links, scripts }
}
/** @param {readonly string[]} urls @param {string} base @param {AbortSignal} signal */
async function assets(urls, base, signal) {
  const siblings = new AbortController()
  const requests = urls.map((url) =>
    request(
      new URL(url.replaceAll('&amp;', '&'), base),
      AbortSignal.any([signal, siblings.signal]),
    ),
  )
  try {
    return await Promise.all(requests)
  } finally {
    siblings.abort()
    await Promise.allSettled(requests)
  }
}
module.exports = { request, surface }
