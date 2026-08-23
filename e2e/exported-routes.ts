import { readdirSync, readFileSync } from 'node:fs'
import { join, relative, resolve, sep } from 'node:path'

/**
 * Which bundler produced the artifact under test. `vinext` (the deployed
 * build) is the default; `next` is the CI-only build that keeps
 * `@devup-ui/next-plugin`'s Turbopack production path covered by this same
 * suite. Both must satisfy every assertion in `e2e/`.
 */
export const LANDING_BUILD_MODE =
  process.env.LANDING_BUILD_MODE === 'next' ? 'next' : 'vinext'

export const LANDING_OUTPUT_ROOT = resolve(
  process.cwd(),
  process.env.LANDING_OUTPUT_ROOT ??
    join(
      'apps',
      'landing',
      ...(LANDING_BUILD_MODE === 'next' ? ['out'] : ['dist', 'client']),
    ),
)

export const EXPECTED_EXPORTED_ROUTES = [
  '/',
  '/components/bottom-sheet',
  '/components/button',
  '/components/checkbox',
  '/components/color-picker',
  '/components/confirm',
  '/components/date-picker',
  '/components/dropdown',
  '/components/footer',
  '/components/header',
  '/components/label',
  '/components/menu',
  '/components/overview',
  '/components/pagination',
  '/components/progress-bar',
  '/components/radio',
  '/components/search',
  '/components/select',
  '/components/slider',
  '/components/snackbar',
  '/components/stepper',
  '/components/tab',
  '/components/textarea',
  '/components/textbox',
  '/components/theme-button',
  '/components/toggle',
  '/components/tooltip',
  '/components/uploader',
  '/docs/api/box',
  '/docs/api/button',
  '/docs/api/center',
  '/docs/api/css',
  '/docs/api/custom-shorthands',
  '/docs/api/flex',
  '/docs/api/grid',
  '/docs/api/group-selector',
  '/docs/api/image',
  '/docs/api/input',
  '/docs/api/selector',
  '/docs/api/style-props',
  '/docs/api/text',
  '/docs/api/v-stack',
  '/docs/core-concepts/nm-base',
  '/docs/core-concepts/no-dependencies',
  '/docs/core-concepts/optimize-css',
  '/docs/core-concepts/style-storage',
  '/docs/core-concepts/type-inference-system',
  '/docs/core-concepts/zero-runtime',
  '/docs/devup/breakpoints',
  '/docs/devup/colors',
  '/docs/devup/devup-json',
  '/docs/devup/figma-plugin',
  '/docs/devup/length',
  '/docs/devup/shadow',
  '/docs/devup/typography',
  '/docs/features',
  '/docs/figma-and-theme-integration/devup-figma-plugin',
  '/docs/figma-and-theme-integration/devup-json',
  '/docs/installation',
  '/docs/overview',
  '/docs/quick-start',
  '/showcase',
  '/team',
] as const

/**
 * Not-found documents are artifacts of the static host contract, not routable
 * pages, so they are asserted separately from `EXPECTED_EXPORTED_ROUTES`.
 * Next's export additionally emits `_not-found.html` next to `404.html`;
 * vinext emits `404.html` alone.
 */
export const NOT_FOUND_HTML_FILES =
  LANDING_BUILD_MODE === 'next' ? ['404.html', '_not-found.html'] : ['404.html']

export const EXPECTED_EXPORTED_HTML_FILES = [
  ...NOT_FOUND_HTML_FILES,
  ...EXPECTED_EXPORTED_ROUTES.map((route) =>
    route === '/' ? 'index.html' : `${route.slice(1)}.html`,
  ),
].sort()

function walkFiles(directory: string): string[] {
  return readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    const path = join(directory, entry.name)
    return entry.isDirectory() ? walkFiles(path) : path
  })
}

function toOutputPath(path: string): string {
  return relative(LANDING_OUTPUT_ROOT, path).split(sep).join('/')
}

export function getExportedHtmlFiles(): string[] {
  return walkFiles(LANDING_OUTPUT_ROOT)
    .filter((path) => path.endsWith('.html'))
    .map(toOutputPath)
    .filter((path) => !path.startsWith('storybook/'))
    .sort()
}

export function getExportedRoutes(): string[] {
  return getExportedHtmlFiles()
    .filter((path) => !NOT_FOUND_HTML_FILES.includes(path))
    .map((path) =>
      path === 'index.html' ? '/' : `/${path.replace(/\.html$/, '')}`,
    )
    .sort()
}

export function getLocalAssetPaths(): string[] {
  const assetPaths = new Set<string>()
  const files = walkFiles(LANDING_OUTPUT_ROOT)

  for (const path of files) {
    if (!path.endsWith('.html') && !path.endsWith('.css')) continue

    const content = readFileSync(path, 'utf8')
    const htmlAssetPattern =
      /<(?:img|link|script|source)\b[^>]*?\b(?:href|src)=["']([^"'<>]+)["']/giu
    const cssAssetPattern = /url\(\s*["']?(\/[^)'"\s]+)["']?\s*\)/giu

    for (const match of content.matchAll(htmlAssetPattern)) {
      if (match[1].startsWith('/')) assetPaths.add(match[1])
    }

    for (const match of content.matchAll(cssAssetPattern)) {
      assetPaths.add(match[1])
    }
  }

  return [...assetPaths].sort()
}
