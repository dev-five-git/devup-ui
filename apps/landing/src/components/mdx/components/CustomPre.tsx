import { Box, css } from '@devup-ui/react'

export function CustomPre({ children }: { children: React.ReactNode }) {
  return (
    <Box
      as="pre"
      className={css({
        margin: '0',
        w: '100%',
        whiteSpace: 'pre-wrap',
        lineBreak: 'anywhere',
        bg: 'transparent',
        overflow: 'auto',
      })}
      selectors={{
        '& pre': {
          margin: '0',
          w: '100%',
          whiteSpace: 'pre-wrap',
          lineBreak: 'anywhere',
          bg: 'transparent',
          overflow: 'auto',
        },
        '& pre, & code, & span, & p': {
          margin: '0',
          w: '100%',
          whiteSpace: 'pre-wrap',
          lineBreak: 'anywhere',
          bg: 'transparent',
          overflow: 'auto',
        },
      }}
    >
      {children}
    </Box>
  )
}
