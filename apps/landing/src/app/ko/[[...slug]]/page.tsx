import type { Metadata } from 'next'
import { notFound } from 'next/navigation'

import { localizedMetadata } from '@/i18n/metadata'
import { pages, translations } from '@/i18n/pages.generated'
import { englishRoutes, translatedRoutes } from '@/i18n/routes.generated'

import ComponentPage, {
  generateMetadata as componentMetadata,
} from '../../(detail)/components/[component]/page'
import ComponentsLayout from '../../(detail)/components/layout'
import DocsLayout from '../../(detail)/docs/layout'
import TeamLayout from '../../(detail)/team/layout'

interface KoreanPageProps {
  params: Promise<{ slug?: string[] }>
}

export const dynamicParams = false

export function generateStaticParams() {
  return englishRoutes.map((path) => ({
    slug: path === '/' ? [] : path.slice(1).split('/'),
  }))
}

export async function generateMetadata({
  params,
}: KoreanPageProps): Promise<Metadata> {
  const { slug = [] } = await params
  const path = `/${slug.join('/')}`
  const load = translations[path] ?? pages[path]
  if (
    !load &&
    englishRoutes.some((route) => route === path) &&
    path.startsWith('/components/')
  ) {
    const component = slug.at(-1)!
    return localizedMetadata(
      await componentMetadata({ params: Promise.resolve({ component }) }),
      path,
      translatedRoutes.includes(path),
    )
  }
  if (!load) notFound()
  const page = await load()
  return localizedMetadata(page.metadata ?? {}, path, !!translations[path])
}

export default async function KoreanPage({ params }: KoreanPageProps) {
  const { slug = [] } = await params
  const path = `/${slug.join('/')}`
  const load = translations[path] ?? pages[path]
  if (
    !load &&
    englishRoutes.some((route) => route === path) &&
    path.startsWith('/components/')
  ) {
    return (
      <ComponentsLayout>
        <ComponentPage
          locale="ko"
          params={Promise.resolve({ component: slug.at(-1)! })}
        />
      </ComponentsLayout>
    )
  }
  if (!load) notFound()
  const { default: Content } = await load()
  const content = (
    <div lang={translations[path] ? 'ko' : 'en'}>
      <Content />
    </div>
  )

  if (path.startsWith('/docs/')) return <DocsLayout>{content}</DocsLayout>
  if (path.startsWith('/components/'))
    return <ComponentsLayout>{content}</ComponentsLayout>
  if (path === '/team') return <TeamLayout>{content}</TeamLayout>
  return content
}
