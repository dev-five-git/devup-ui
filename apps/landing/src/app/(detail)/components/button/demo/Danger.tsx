import { Button } from '@devup-ui/components'
import { Box, css } from '@devup-ui/react'

/**
 * **Danger**
 * Use `danger` prop to signal caution.
 */
export function Danger() {
  return (
    <Box w="100%">
      <Box display="flex" flexWrap="wrap" gap="12px" marginBottom="16px">
        <Button className={css({ h: 'min-content' })} danger size="sm">
          Danger sm
        </Button>
        <Button className={css({ h: 'min-content' })} danger size="md">
          Danger md
        </Button>
        <Button className={css({ h: 'min-content' })} danger size="lg">
          Danger lg
        </Button>
      </Box>
    </Box>
  )
}
