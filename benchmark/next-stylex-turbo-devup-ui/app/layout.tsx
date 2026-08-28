import './globals.css'

import type { Metadata } from 'next'

export const metadata: Metadata = {
  title: 'StyleX Turbopack Devup UI benchmark',
  description: 'StyleX benchmark built with Next.js Turbopack and Devup UI',
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
