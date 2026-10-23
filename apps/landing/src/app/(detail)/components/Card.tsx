import { VStack } from '@devup-ui/react'

export default function Card({
  children,
  className,
  ...props
}: React.ComponentProps<'div'>) {
  return (
    <VStack
      _active={{
        boxShadow: 'none',
        transform: 'scale(0.95)',
      }}
      _hover={{
        boxShadow: '0 0 20px 0 rgba(0, 0, 0, 0.15)',
      }}
      bg="$containerBackground"
      border="1px solid $border"
      borderRadius="10px"
      className={className}
      cursor="pointer"
      styleOrder={1}
      transition="all 0.2s ease"
      {...props}
    >
      {children}
    </VStack>
  )
}
