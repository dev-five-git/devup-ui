import type { Metadata } from 'next'

export const metadata: Metadata = {
  title: 'Vinext Devup UI Benchmark',
  description: 'Benchmark for devup-ui on vinext',
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
