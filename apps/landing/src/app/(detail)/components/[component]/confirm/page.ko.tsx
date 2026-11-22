import {
  ComponentDoc,
  getComponentMetadata,
} from '@/app/(detail)/components/ComponentDoc'

import Api from './Api.ko.mdx'
import Index from './index.ko.mdx'

export const metadata = getComponentMetadata('confirm', 'ko')

export default function Page() {
  return (
    <ComponentDoc Api={Api} component="confirm" Index={Index} locale="ko" />
  )
}
