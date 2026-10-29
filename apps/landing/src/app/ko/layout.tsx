import {
  metadata as siteMetadata,
  SiteLayout,
} from '@/components/layout/SiteLayout'

export const metadata = siteMetadata

export default function KoreanLayout({
  children,
}: {
  children: React.ReactNode
}) {
  return <SiteLayout locale="ko">{children}</SiteLayout>
}
