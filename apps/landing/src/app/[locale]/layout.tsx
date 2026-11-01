import {
  metadata as siteMetadata,
  SiteLayout,
} from '@/components/layout/SiteLayout'

export const metadata = siteMetadata
export const dynamicParams = false

export function generateStaticParams() {
  return [{ locale: 'ko' }]
}

export default async function LocalizedLayout({
  children,
  params,
}: {
  children: React.ReactNode
  params: Promise<{ locale: string }>
}) {
  const { locale } = await params
  // dynamicParams rejects unsupported locales. The 404 shell uses English.
  return (
    <SiteLayout locale={locale === 'ko' ? 'ko' : 'en'}>{children}</SiteLayout>
  )
}
