import { Box, Flex, Text } from '@devup-ui/react'
import { type ComponentProps, forwardRef } from 'react'

export interface ProgressBarProps extends Omit<
  ComponentProps<'div'>,
  'children' | 'role'
> {
  /** How much is done, between 0 and `max` */
  value: number
  /** The value that means done; the number of steps for the `step` variant */
  max?: number
  /** One continuous bar, or one segment per step */
  variant?: 'bar' | 'step'
  /** Shows the progress above the bar: `3/6` for steps, `50%` for a bar */
  showValue?: boolean
  classNames?: {
    track?: string
    indicator?: string
    value?: string
  }
  colors?: {
    primary?: string
    track?: string
    text?: string
  }
}

const PRIMARY = 'var(--primary, light-dark(#674DC7, #8163E1))'
const TRACK = 'var(--track, light-dark(#E4E4E4, #434343))'

/** Progress of a task or a multi-step flow, exposed as a `progressbar` */
export const ProgressBar = forwardRef<HTMLDivElement, ProgressBarProps>(
  function ProgressBar(
    {
      value,
      max: maxProp = 100,
      variant = 'bar',
      showValue = false,
      classNames,
      colors,
      ...props
    },
    ref,
  ) {
    const isStep = variant === 'step'
    // A step count is whole; a non-positive or missing max would divide by zero
    const max = Math.max(isStep ? Math.floor(maxProp) : maxProp, 0) || 1
    const current = Math.min(Math.max(value || 0, 0), max)
    const filledSteps = Math.floor(current)
    const percent = (current / max) * 100

    return (
      <Box
        ref={ref}
        aria-valuemax={max}
        aria-valuemin={0}
        aria-valuenow={current}
        role="progressbar"
        styleVars={{
          primary: colors?.primary,
          track: colors?.track,
          text: colors?.text,
        }}
        {...props}
      >
        {showValue && (
          <Text
            aria-hidden
            className={classNames?.value}
            color="var(--text, light-dark(#8D8C9A, #A9A8AB))"
            display="block"
            fontSize="14px"
            lineHeight="1.4"
            mb="8px"
            styleOrder={1}
            textAlign="right"
          >
            {isStep ? `${filledSteps}/${max}` : `${Math.round(percent)}%`}
          </Text>
        )}
        {isStep ? (
          <Flex gap="8px">
            {Array.from({ length: max }, (_, index) => (
              <Box
                key={index}
                bg={index < filledSteps ? PRIMARY : TRACK}
                borderRadius="4px"
                className={
                  index < filledSteps
                    ? classNames?.indicator
                    : classNames?.track
                }
                data-filled={index < filledSteps || undefined}
                flex="1"
                h="8px"
                styleOrder={1}
                transition="background-color 0.2s ease-in-out"
              />
            ))}
          </Flex>
        ) : (
          <Box
            bg={TRACK}
            borderRadius="4px"
            className={classNames?.track}
            h="8px"
            overflow="hidden"
            styleOrder={1}
          >
            <Box
              bg={PRIMARY}
              borderRadius="4px"
              className={classNames?.indicator}
              h="100%"
              styleOrder={1}
              transition="width 0.2s ease-in-out"
              w={`${percent}%`}
            />
          </Box>
        )}
      </Box>
    )
  },
)
