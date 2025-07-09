import { Button } from '@devup-ui/components'
import { Box, css } from '@devup-ui/react'

/**
 * **Variant & Size**
 * `Button` components has `default` and `primary` variants. `Size` prop determines the paddings of the button.
 */
export function Variants() {
  return (
    <>
      <Box display="flex" flexWrap="wrap" gap="12px" marginBottom="16px">
        <Button
          className={css({ height: 'min-content' })}
          size="sm"
          variant="primary"
        >
          Primary sm
        </Button>
        <Button
          className={css({ height: 'min-content' })}
          size="md"
          variant="primary"
        >
          Primary md
        </Button>
        <Button
          className={css({ height: 'min-content' })}
          size="lg"
          variant="primary"
        >
          Primary lg
        </Button>
      </Box>
      <Box display="flex" flexWrap="wrap" gap="12px" marginBottom="16px">
        <Button className={css({ height: 'min-content' })} size="sm">
          Default sm
        </Button>
        <Button className={css({ height: 'min-content' })} size="md">
          Default md
        </Button>
        <Button className={css({ height: 'min-content' })} size="lg">
          Default lg
        </Button>
      </Box>
    </>
  )
}
