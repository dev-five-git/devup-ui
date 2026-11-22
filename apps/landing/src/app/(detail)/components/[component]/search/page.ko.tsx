import {
  ComponentDoc,
  getComponentMetadata,
} from '@/app/(detail)/components/ComponentDoc'

import Api from './Api.ko.mdx'
import Index from './index.ko.mdx'

export const metadata = getComponentMetadata('search', 'ko')

export default function Page() {
  return <ComponentDoc Api={Api} component="search" Index={Index} locale="ko" />
}
