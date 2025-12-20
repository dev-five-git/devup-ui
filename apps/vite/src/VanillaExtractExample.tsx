import { useState } from 'react'

import * as styles from './VanillaExtractExample.css'

export function VanillaExtractExample() {
  const [count, setCount] = useState(0)

  return (
    <div className={styles.container}>
      <h1 className={styles.title}>Vanilla Extract Example</h1>

      <div className={styles.card}>
        <h2 className={styles.text}>
          This component is styled using Vanilla Extract!
        </h2>
        <p className={styles.text}>
          Vanilla Extract provides type-safe, zero-runtime CSS-in-JS with
          excellent developer experience. All styles are extracted at build
          time.
        </p>
      </div>

      <div className={styles.card}>
        <p className={styles.text}>Count: {count}</p>
        <button className={styles.button} onClick={() => setCount(count + 1)}>
          Increment
        </button>
      </div>

      <div className={styles.card}>
        <p className={styles.text}>Features demonstrated:</p>
        <ul>
          <li>Type-safe styles with TypeScript</li>
          <li>Zero runtime overhead</li>
          <li>Hover effects and transitions</li>
          <li>Build-time CSS extraction</li>
        </ul>
      </div>
    </div>
  )
}
