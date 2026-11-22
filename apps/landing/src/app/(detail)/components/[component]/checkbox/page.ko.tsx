import {
  ComponentDoc,
  getComponentMetadata,
} from '@/app/(detail)/components/ComponentDoc'

import Api from './Api.ko.mdx'
import Index from './index.ko.mdx'

export const metadata = getComponentMetadata('checkbox', 'ko')

export default function Page() {
  return (
    <ComponentDoc Api={Api} component="checkbox" Index={Index} locale="ko" />
  )
}
