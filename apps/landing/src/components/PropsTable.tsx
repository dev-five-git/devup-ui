import { Box, Text, VStack } from '@devup-ui/react'
import Markdown from 'react-markdown'

import { _components } from '@/mdx-components'

import { CustomCodeBlock } from './mdx/components/CustomCodeBlock'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeaderCell,
  TableRow,
} from './mdx/components/Table'

interface ComponentProp {
  property: string
  description?: string
  type?: string
  default?: string
}

const MdxComponentsWithCodeBlock = ({ children }: { children?: string }) => {
  return (
    <Markdown
      components={{
        ...(_components as any),
        code: CustomCodeBlock,
      }}
    >
      {children}
    </Markdown>
  )
}

interface PropTableProps {
  componentProps: ComponentProp[]
}

export const PropsTable = async (props: PropTableProps) => {
  const { componentProps } = props

  return (
    <Box maxWidth="100%" overflow="auto" width="100%">
      <Table border={0} style={{ minWidth: '600px' }}>
        <TableHead>
          <TableRow>
            <TableHeaderCell>Prop</TableHeaderCell>
            <TableHeaderCell>description</TableHeaderCell>
            <TableHeaderCell>Type</TableHeaderCell>
            <TableHeaderCell>Default</TableHeaderCell>
          </TableRow>
        </TableHead>
        <TableBody>
          {componentProps.length === 0 && (
            <TableRow>
              <TableCell colSpan={3}>
                <Text>No props to display</Text>
              </TableCell>
            </TableRow>
          )}
          {componentProps.map(
            ({ property, description, type, default: defaultValue }) => (
              <TableRow key={property}>
                <TableCell>
                  <Text typography="bodyBold">{property}</Text>
                </TableCell>
                <TableCell>
                  <MdxComponentsWithCodeBlock>
                    {description}
                  </MdxComponentsWithCodeBlock>
                </TableCell>
                <TableCell>
                  <VStack>
                    <MdxComponentsWithCodeBlock>
                      {type?.replaceAll('"', "'")}
                    </MdxComponentsWithCodeBlock>
                  </VStack>
                </TableCell>
                <TableCell>
                  <MdxComponentsWithCodeBlock>
                    {defaultValue}
                  </MdxComponentsWithCodeBlock>
                </TableCell>
              </TableRow>
            ),
          )}
        </TableBody>
      </Table>
    </Box>
  )
}
