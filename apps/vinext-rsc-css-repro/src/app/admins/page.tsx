import { PageTitle } from '../../components/PageTitle'

export default function AdminsPage() {
  return (
    <main>
      <PageTitle right={<button type="button">Create</button>}>
        Admins
      </PageTitle>
    </main>
  )
}
