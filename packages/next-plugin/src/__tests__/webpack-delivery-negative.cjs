const assert = require('node:assert/strict')
const { once } = require('node:events')
const { createServer } = require('node:http')
const { createRequire } = require('node:module')
const fromNext = createRequire(require.resolve('next/package.json'))
const delivery = require('./webpack-delivery-http.cjs')
const failures = {
  'document-status': /HTML status/,
  'css-status': /stylesheet \/proof.css/,
  'empty-css': /empty CSS/,
  'unbound-class': /actual class \.proof has no linked declarations/,
  'wrong-value': /missing\/wrong background/,
}
async function main() {
  for (const [failure, message] of Object.entries(failures)) {
    const server = createServer((request, response) => {
      const css = request.url === '/proof.css'
      response.statusCode =
        (css && failure === 'css-status') ||
        (!css && failure === 'document-status')
          ? 404
          : 200
      response.setHeader('Content-Type', css ? 'text/css' : 'text/html')
      response.end(
        css
          ? failure === 'empty-css'
            ? ''
            : failure === 'unbound-class'
              ? '.other{background:maroon;padding:13px}'
              : '.proof{background:cyan;padding:13px}'
          : '<html><head><link rel="stylesheet" href="/proof.css"></head><body><div id="server-proof" class="proof"></div></body></html>',
      )
    })
    server.listen(0, '127.0.0.1')
    await once(server, 'listening')
    try {
      await assert.rejects(
        delivery(
          `http://127.0.0.1:${server.address().port}`,
          fromNext('postcss').parse,
        ),
        message,
      )
      console.info('HTTP-REJECTION ' + failure)
    } finally {
      await new Promise((resolve, reject) =>
        server.close((error) => (error ? reject(error) : resolve())),
      )
    }
  }
}
main().catch((error) => {
  console.error(error)
  process.exitCode = 1
})
