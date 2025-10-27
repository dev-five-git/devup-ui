import { Text, VStack } from '@devup-ui/react'
import Markdown from 'react-markdown'

import { _components } from '@/mdx-components'

import { CustomCodeBlock } from './mdx/components/CustomCodeBlock'
import { TableBody } from './TableBody'
import { TableCell } from './TableCell'
import { TableHead } from './TableHead'
import { TableHeaderCell } from './TableHeaderCell'
import { TableRoot } from './TableRoot'
import { TableRow } from './TableRow'

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
    <TableRoot border={0}>
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
    </TableRoot>
  )
}
