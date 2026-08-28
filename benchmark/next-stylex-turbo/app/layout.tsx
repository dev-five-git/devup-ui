import './globals.css'

import type { Metadata } from 'next'

export const metadata: Metadata = {
  title: 'StyleX Turbopack benchmark',
  description: 'StyleX benchmark built with Next.js Turbopack',
}

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode
}>) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  )
}
