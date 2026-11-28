import {
  ComponentDoc,
  getComponentMetadata,
} from '@/app/(detail)/components/ComponentDoc'

import Api from './Api.ko.mdx'
import Index from './index.ko.mdx'

export const metadata = getComponentMetadata('date-picker', 'ko')

export default function Page() {
  return (
    <ComponentDoc Api={Api} component="date-picker" Index={Index} locale="ko" />
  )
}
