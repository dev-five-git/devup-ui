import { css } from '@devup-ui/react'

import Card from './Card'

export default function MdxCard({ children }: { children: React.ReactNode }) {
  return (
    <Card
      className={css({
        _active: {
          transform: 'none',
        },
        _hover: {
          boxShadow: 'none',
        },
        borderRadius: '10px',
        border: '1px solid $border',
        bg: '$containerBackground',
        maxWidth: '100%',
        minWidth: '300px',
        flexShrink: 0,
        cursor: 'default',
        marginBottom: '20px',
        _lastChild: {
          marginBottom: '0',
        },
      })}
    >
      {children}
    </Card>
  )
}
