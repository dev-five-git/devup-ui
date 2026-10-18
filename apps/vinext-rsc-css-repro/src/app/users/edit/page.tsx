'use client'

import { useState } from 'react'

import { PageTitle } from '../../../components/PageTitle'

export default function UserEditPage() {
  const [name, setName] = useState('')
  return (
    <main>
      <PageTitle>Edit user</PageTitle>
      <input
        aria-label="name"
        onChange={(event) => setName(event.target.value)}
        value={name}
      />
    </main>
  )
}
