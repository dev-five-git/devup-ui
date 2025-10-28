'use client'

import { useState } from 'react'

export default function HomePage() {
  const [color, setColor] = useState('text-yellow-500')
  const [enabled, setEnabled] = useState(false)

  return (
    <div>
      <p className="bg-blue-500 text-white p-2">Track & field champions:</p>
      <section
        className={`${color} cursor-pointer relative py-7 hover:bg-yellow-500 hover:cursor-cell text-3xl`}
        data-testid="box"
      >
        <div>hello</div>
        <div>hello</div>
      </section>
      <p className="text-gray-900">text</p>
      <div
        className={`${enabled ? 'text-green-500' : 'text-blue-500'} text-3xl pr-5`}
      >
        hello
      </div>
      <div className="text-xs sm:text-3xl">hello</div>
      <button
        className="bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded mt-4"
        onClick={() => {
          setColor('text-blue-500')
          setEnabled((prev) => !prev)
        }}
      >
        Change
      </button>
    </div>
  )
}
