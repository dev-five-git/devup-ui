import 'sanitize.css'

import { Flex } from '@devup-ui/react'

import { DetailHeader } from '../../components/DetailHeader'
import { LeftMenu } from './LeftMenu'

export default function DetailLayout({
  children,
}: Readonly<{
  children: React.ReactNode
}>) {
  return (
    <>
      <DetailHeader />
      <Flex>
        <LeftMenu />
        {children}
      </Flex>
    </>
  )
}
