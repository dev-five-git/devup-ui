import { Box } from '@devup-ui/react'

export default function TeamLayout({
  children,
}: Readonly<{
  children: React.ReactNode
}>) {
  return (
    <>
      <Box
        maxW="1014px"
        minH="calc(100vh - 500px)"
        mx="auto"
        px={['16px', null, '60px']}
        py={['20px', null, '40px']}
      >
        {children}
      </Box>
    </>
  )
}
