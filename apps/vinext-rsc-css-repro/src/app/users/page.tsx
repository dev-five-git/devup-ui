import Link from 'next/link'

import { PageTitle } from '../../components/PageTitle'

export default function UsersPage() {
  return (
    <main>
      <PageTitle right={<Link href="/users/edit/">Create</Link>}>
        Users
      </PageTitle>
    </main>
  )
}
