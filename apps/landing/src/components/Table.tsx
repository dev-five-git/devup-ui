import {
  Box,
  type DevupComponentBaseProps,
  type DevupComponentProps,
} from '@devup-ui/react'

// FIXME: Merge type is not exported in @devup-ui/react
type Merge<T, U> = Omit<T, Extract<keyof T, keyof U>> & U

type TableComponentProps<T extends React.ElementType> = Merge<
  DevupComponentBaseProps<T>,
  DevupComponentProps<T>
>

const TableRoot = ({ ...props }: TableComponentProps<'table'>) => {
  return (
    <Box borderRadius="0.5rem" overflow="hidden">
      <Box as="table" borderCollapse="collapse" borderSpacing={0} {...props} />
    </Box>
  )
}

const TableHead = ({ ...props }: TableComponentProps<'thead'>) => {
  return (
    <Box
      as="thead"
      {...props}
      selectors={{
        '& tr': {
          bg: '$cardBg',
        },
      }}
    />
  )
}

const TableBody = ({ ...props }: TableComponentProps<'tbody'>) => {
  return <Box as="tbody" {...props}></Box>
}

const TableRow = ({ ...props }: TableComponentProps<'tr'>) => {
  return (
    <Box
      as="tr"
      borderBottom="1px solid var(--border, #E4E4E4)"
      selectors={{
        '& + &:last-of-type': {
          borderBottom: 'none',
        },
      }}
      {...props}
    />
  )
}

const TableCell = ({ ...props }: TableComponentProps<'td'>) => {
  return <Box as="td" padding="0.5rem 1rem" {...props} />
}

const TableHeaderCell = ({ ...props }: TableComponentProps<'th'>) => {
  return <Box as="th" padding="0.5rem 1rem" textAlign="left" {...props} />
}

export { TableBody, TableCell, TableHead, TableHeaderCell, TableRoot, TableRow }

export function Table() {
  return <></>
}
