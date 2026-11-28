import {
  ComponentDoc,
  getComponentMetadata,
} from '@/app/(detail)/components/ComponentDoc'

import Api from './Api.ko.mdx'
import Index from './index.ko.mdx'

export const metadata = getComponentMetadata('button', 'ko')

export default function Page() {
  return <ComponentDoc Api={Api} component="button" Index={Index} locale="ko" />
}
