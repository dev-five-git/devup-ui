import { Box, css } from '@devup-ui/react'

import { IconCheck } from './IconCheck'

interface CheckboxProps {
  isChecked: boolean
}

export function Checkbox({ isChecked }: CheckboxProps) {
  return (
    <Box
      bg={isChecked ? '$primary' : '$border'}
      borderRadius="4px"
      boxSize="18px"
      pos="relative"
      transition="background-color 0.1s ease-in-out"
    >
      {isChecked && (
        <IconCheck
          className={css({
            position: 'absolute',
            top: '55%',
            left: '50%',
            transform: 'translate(-50%, -50%)',
          })}
        />
      )}
    </Box>
  )
}
