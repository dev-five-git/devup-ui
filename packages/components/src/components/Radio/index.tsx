import { Box, Input, Text } from '@devup-ui/react'

type RadioProps = Omit<React.InputHTMLAttributes<HTMLInputElement>, 'type'> & {
  checked?: boolean
  classNames?: {
    label?: string
  }
  color?: string
  hoverColor?: string
  styles?: {
    label?: React.CSSProperties
  }
  colors?: {
    primary?: string
    border?: string
    text?: string
    bg?: string
    hoverBg?: string
    hoverBorder?: string
    hoverColor?: string
    checkedBg?: string
    checkedBorder?: string
    checkedColor?: string
    disabledBg?: string
    disabledColor?: string
  }
  variant?: 'default' | 'button'
} & (
    | {
        variant?: 'default'
        firstButton?: undefined
        lastButton?: undefined
      }
    | {
        variant: 'button'
        firstButton?: boolean
        lastButton?: boolean
      }
  )

export function Radio({
  className,
  disabled,
  children,
  variant = 'default',
  checked,
  classNames,
  styles,
  style,
  firstButton,
  lastButton,
  colors,
  ...props
}: RadioProps) {
  const isButton = variant === 'button'
  return (
    <Box
      alignItems={isButton ? undefined : 'center'}
      aria-disabled={disabled}
      as="label"
      cursor={isButton ? undefined : 'pointer'}
      display={isButton ? undefined : 'inline-flex'}
      gap={isButton ? undefined : 2}
      selectors={{
        '&[aria-disabled=true]': {
          cursor: 'default',
        },
      }}
    >
      {isButton ? (
        <Input
          checked={checked}
          className={className}
          data-radio-input
          disabled={disabled}
          display="none"
          opacity={0}
          styleOrder={1}
          type="radio"
          {...props}
        />
      ) : (
        <Input
          _focus={{
            outline: '1px sold var(--border, var(--primary))',
          }}
          appearance="none"
          bg="light-dark(#fff, #2E2E2E)"
          border="1px solid"
          borderColor="$border"
          borderRadius="100%"
          checked={checked}
          className={className}
          data-radio-input
          disabled={disabled}
          height="18px"
          m={0}
          selectors={{
            // checked
            '&:checked:not(:disabled)': {
              bg: 'var(--checkedBg, var(--primary, light-dark(#fff, #2E2E2E)))',
              border: '3px solid',
              borderColor: 'var(--checkedBg, light-dark(#fff, #2E2E2E))',
              boxShadow: '0 0 0 1px var(--checkedBorder, var(--primary))',
            },
            // hover
            '&:hover:not(:disabled,:checked)': {
              border: '1px solid var(--hoverBorder, var(--primary))',
              bg: 'var(--hoverBg, light-dark(color-mix(in srgb, var(--primary) 10%, white 90%), color-mix(in srgb, var(--primary) 10%, black 90%)))',
            },
            // disabled
            '&:is(:disabled, [aria-disabled=true])': {
              bgColor: 'var(--disabledBg, light-dark(#F0F0F3, #47474A))',
            },
          }}
          styleOrder={1}
          styleVars={{
            primary: colors?.primary,
            border: colors?.border,
            text: colors?.text,
            bg: colors?.bg,
            hoverBg: colors?.hoverBg,
            hoverBorder: colors?.hoverBorder,
            hoverColor: colors?.hoverColor,
            checkedBg: colors?.checkedBg,
            checkedBorder: colors?.checkedBorder,
            checkedColor: colors?.checkedColor,
            disabledBg: colors?.disabledBg,
            disabledColor: colors?.disabledColor,
          }}
          transition=".25s"
          type="radio"
          width="18px"
          {...props}
        />
      )}
      {variant === 'button' ? (
        <Box
          _disabled={{
            cursor: 'not-allowed',
          }}
          aria-disabled={disabled}
          bg="var(--bg, light-dark(#fff, #2E2E2E))"
          border="1px solid"
          borderColor="$border"
          borderRadius={
            firstButton ? '6px 0 0 6px' : lastButton ? '0 6px 6px 0' : undefined
          }
          className={className}
          color="var(--text, light-dark(#000, #fff))"
          cursor="pointer"
          data-radio-button
          display="flex"
          px={8}
          py={4}
          selectors={{
            // checked
            '[data-radio-input]:checked + &:not([aria-disabled=true])': {
              fontWeight: 600,
              bg: `var(--checkedBg, light-dark(color-mix(in srgb, var(--primary) 10%, white 80%), color-mix(in srgb, var(--primary) 10%, black 80%)))`,
              borderColor: 'var(--checkedBorder, var(--primary))',
              color: 'var(--checkedColor, var(--primary))',
            },
            // hover
            '&:hover:not([aria-disabled=true])': {
              bg: `var(--hoverBg, light-dark(color-mix(in srgb, var(--primary) 10%, white 90%), color-mix(in srgb, var(--primary) 10%, black 90%)))`,
              borderColor: 'var(--hoverBorder, var(--primary))',
            },
            // disabled
            '[data-radio-input]:disabled + &': {
              bg: 'var(--disabledBg, light-dark(#F0F0F3, #47474A))',
              color: 'var(--disabledColor, light-dark(#D6D7DE, #373737))',
            },
          }}
          style={style}
          styleOrder={1}
          styleVars={{
            primary: colors?.primary,
            border: colors?.border,
            text: colors?.text,
            bg: colors?.bg,
            hoverBg: colors?.hoverBg,
            hoverBorder: colors?.hoverBorder,
            hoverColor: colors?.hoverColor,
            checkedBg: colors?.checkedBg,
            checkedBorder: colors?.checkedBorder,
            checkedColor: colors?.checkedColor,
            disabledBg: colors?.disabledBg,
            disabledColor: colors?.disabledColor,
          }}
          transition="background-color 0.25s, border-color 0.25s"
          w="fit-content"
        >
          {children}
        </Box>
      ) : (
        <Text
          aria-disabled={disabled}
          className={classNames?.label}
          color="var(--text, light-dark(#000, #fff))"
          selectors={{
            '&[aria-disabled=true]': {
              color: 'var(--disabledColor, light-dark(#D6D7DE, #373737))',
            },
          }}
          style={style}
          styleOrder={1}
          styleVars={{
            primary: colors?.primary,
            border: colors?.border,
            text: colors?.text,
            bg: colors?.bg,
            hoverBg: colors?.hoverBg,
            hoverBorder: colors?.hoverBorder,
            hoverColor: colors?.hoverColor,
            checkedBg: colors?.checkedBg,
            checkedBorder: colors?.checkedBorder,
            checkedColor: colors?.checkedColor,
            disabledBg: colors?.disabledBg,
            disabledColor: colors?.disabledColor,
          }}
        >
          {children}
        </Text>
      )}
    </Box>
  )
}
