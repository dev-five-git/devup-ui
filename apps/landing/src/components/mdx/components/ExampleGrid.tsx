import { Grid } from '@devup-ui/react'
import { ReactNode } from 'react'

interface ExampleGridProps {
  children: ReactNode
}

export function ExampleGrid({ children }: ExampleGridProps) {
  return (
    <Grid gap="16px" gridTemplateColumns={['repeat(1, 1fr)', 'repeat(2, 1fr)']}>
      {children}
    </Grid>
  )
}
