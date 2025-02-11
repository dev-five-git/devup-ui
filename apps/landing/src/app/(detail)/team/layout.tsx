import { Box } from '@devup-ui/react'

export default function TeamLayout({
  children,
}: Readonly<{
  children: React.ReactNode
}>) {
  return (
    <>
      <Box maxW="1014px" minH="calc(100vh - 500px)" mx="auto" p="40px 60px">
        {children}
      </Box>
    </>
  )
}
