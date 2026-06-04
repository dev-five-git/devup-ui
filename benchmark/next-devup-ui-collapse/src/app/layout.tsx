import type { Metadata } from 'next'

export const metadata: Metadata = {
  title: 'collapse benchmark',
  description: 'single-importer collapse benchmark',
}

export default function RootLayout({
  children,
}: Readonly<{ children: React.ReactNode }>) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  )
}
