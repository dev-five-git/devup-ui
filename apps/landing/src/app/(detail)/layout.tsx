import {
  metadata as siteMetadata,
  SiteLayout,
} from '@/components/layout/SiteLayout'

export const metadata = siteMetadata

export default function EnglishLayout({
  children,
}: {
  children: React.ReactNode
}) {
  return <SiteLayout>{children}</SiteLayout>
}
