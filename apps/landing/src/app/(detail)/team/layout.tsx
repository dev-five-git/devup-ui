import { Box } from '@devup-ui/react'

export default function TeamLayout({
  children,
}: Readonly<{
  children: React.ReactNode
}>) {
  return (
    <>
      <Box mx="auto" p="40px 60px" w="1014px">
        {children}
      </Box>
    </>
  )
}
