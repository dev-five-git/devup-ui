import {
  ComponentDoc,
  getComponentMetadata,
} from '@/app/(detail)/components/ComponentDoc'

import Api from './Api.ko.mdx'
import Index from './index.ko.mdx'

export const metadata = getComponentMetadata('tab', 'ko')

export default function Page() {
  return <ComponentDoc Api={Api} component="tab" Index={Index} locale="ko" />
}
