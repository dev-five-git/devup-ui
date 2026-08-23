import {
  copyFile,
  cp,
  readdir,
  readFile,
  rm,
  writeFile,
} from 'node:fs/promises'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'

import { staticExportDeploymentId } from './static-export-config.js'
import { installStaticExportRscTransport } from './static-export-rsc-transport.js'

const clientDir = fileURLToPath(new URL('./dist/client/', import.meta.url))
const exportDir = fileURLToPath(new URL('./out/', import.meta.url))
const devupCssProxyPreload =
  /<link\b(?=[^>]*\brel="modulepreload")(?=[^>]*\bhref="\/_next\/static\/chunks\/devup-ui(?:-\d+)?\.css-[^"]+\.js")[^>]*>/gu
const staticExportRscTransport = `<script data-vinext-static-rsc-transport>(${installStaticExportRscTransport.toString()})(${JSON.stringify(staticExportDeploymentId)})</script>`

async function listHtmlFiles(directory) {
  const entries = await readdir(directory, { withFileTypes: true })
  const files = await Promise.all(
    entries.map((entry) => {
      const path = join(directory, entry.name)
      return entry.isDirectory() ? listHtmlFiles(path) : [path]
    }),
  )

  return files.flat().filter((path) => path.endsWith('.html'))
}

await rm(exportDir, { force: true, recursive: true })
await cp(clientDir, exportDir, { recursive: true })

// Devup's CSS proxy modules are extracted to real stylesheets. Vinext includes
// the removed proxy modules in static HTML preload hints even though no runtime
// chunk imports them, so discard those dead hints from the exported documents.
// The RSC transport lets vinext consume the `.rsc` files from a plain static
// host, which cannot vary clean route URLs by request header.
for (const path of await listHtmlFiles(exportDir)) {
  const html = await readFile(path, 'utf8')
  const sanitizedHtml = html
    .replace(devupCssProxyPreload, '')
    .replace('</head>', `${staticExportRscTransport}</head>`)
  if (sanitizedHtml !== html) await writeFile(path, sanitizedHtml)
}

await copyFile(
  new URL('./out/404.html', import.meta.url),
  new URL('./out/_not-found.html', import.meta.url),
)
