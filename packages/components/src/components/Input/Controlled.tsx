import { useState } from 'react'

import { Input } from '.'

export function Controlled() {
  const [value, setValue] = useState('d')

  return <Input onChange={(e) => setValue(e.target.value)} value={value} />
}
