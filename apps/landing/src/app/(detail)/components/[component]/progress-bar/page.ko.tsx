import {
  ComponentDoc,
  getComponentMetadata,
} from '@/app/(detail)/components/ComponentDoc'

import Api from './Api.ko.mdx'
import Index from './index.ko.mdx'

export const metadata = getComponentMetadata('progress-bar', 'ko')

export default function Page() {
  return (
    <ComponentDoc
      Api={Api}
      component="progress-bar"
      Index={Index}
      locale="ko"
    />
  )
}
