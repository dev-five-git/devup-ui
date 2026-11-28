import {
  ComponentDoc,
  getComponentMetadata,
} from '@/app/(detail)/components/ComponentDoc'

import Api from './Api.ko.mdx'
import Index from './index.ko.mdx'

export const metadata = getComponentMetadata('color-picker', 'ko')

export default function Page() {
  return (
    <ComponentDoc
      Api={Api}
      component="color-picker"
      Index={Index}
      locale="ko"
    />
  )
}
