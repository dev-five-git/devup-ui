import { styled, Text } from '@devup-ui/react'

/** Styled the same in every theme, so it compiles without the runtime */
const Pill = styled(Text)({
  bg: '#5a44ff',
  borderRadius: '999px',
  color: '#fff',
  fontSize: '12px',
  fontWeight: '600',
  px: 2,
  py: 1,
})

export function Badge({ label }: { label: string }) {
  return <Pill>{label}</Pill>
}
