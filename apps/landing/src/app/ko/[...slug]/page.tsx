import { globSync } from 'node:fs'
import { resolve } from 'node:path'

import { COMPONENT_GROUPS } from '@/constants'

export const dynamicParams = false

export function generateStaticParams() {
  const app = resolve(process.cwd(), 'src/app')
  const paths = (directory: string) =>
    [...globSync('**/page.{mdx,tsx}', { cwd: resolve(app, directory) })]
      .map((file) => file.replaceAll('\\', '/'))
      .filter((file) => !file.includes('['))
      .map((file) => file.replace(/(?:^|\/)page\.(mdx|tsx)$/u, ''))
  const translated = new Set(paths('ko'))
  return [
    ...paths('(detail)'),
    ...Object.values(COMPONENT_GROUPS)
      .flat()
      .map((name) => 'components/' + name),
  ]
    .filter((path) => path && !translated.has(path))
    .map((path) => ({ slug: path.split('/') }))
}

export default async function EnglishFallbackPage({
  params,
}: {
  params: Promise<{ slug?: string[] }>
}) {
  const { slug = [] } = await params
  const href = '/' + slug.join('/')
  // A static host cannot run middleware. HTML refresh also works without JS.
  return (
    <>
      <meta content={'0;url=' + href} httpEquiv="refresh" />
      <meta content="noindex" name="robots" />
      <a href={href}>Continue to the English page</a>
    </>
  )
}
