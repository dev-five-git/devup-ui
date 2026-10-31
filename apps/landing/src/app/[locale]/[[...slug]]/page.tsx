import { globSync } from 'node:fs'
import { resolve } from 'node:path'

import { Fragment } from 'react'

import { COMPONENT_GROUPS } from '@/constants'
import { translatedPages } from '@/i18n/pages'

interface LocalizedPageProps {
  params: Promise<{ locale: string; slug?: string[] }>
}

export const dynamicParams = false

export function generateStaticParams() {
  const pages = globSync('**/page.{mdx,tsx}', {
    cwd: resolve(process.cwd(), 'src/app/(detail)'),
  })
  return [
    ...[...pages]
      .map((file) => file.replaceAll('\\', '/'))
      .filter((file) => !file.includes('['))
      .map((file) => file.replace(/(?:^|\/)page\.(mdx|tsx)$/u, '')),
    ...Object.values(COMPONENT_GROUPS)
      .flat()
      .map((name) => `components/${name}`),
  ].map((path) => ({ slug: path ? path.split('/') : [] }))
}

export async function generateMetadata({ params }: LocalizedPageProps) {
  const { slug = [] } = await params
  const page = await translatedPages[`/${slug.join('/')}`]?.load()
  return page?.metadata ?? { robots: { index: false } }
}

export default async function LocalizedPage({ params }: LocalizedPageProps) {
  const { slug = [] } = await params
  const href = `/${slug.join('/')}`
  const translation = translatedPages[href]
  if (!translation) {
    // HTML refresh supports the static host and browsers without JavaScript.
    return (
      <>
        <meta content={`0;url=${href}`} httpEquiv="refresh" />
        <a href={href}>Continue to the English page</a>
      </>
    )
  }
  const { default: Content } = await translation.load()
  const Layout = translation.Layout ?? Fragment
  return (
    <Layout>
      <Content />
    </Layout>
  )
}
