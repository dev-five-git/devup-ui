function cartesianProduct<T extends any[][]>(arrays: T) {
  return arrays.reduce(
    (acc, curr) => acc.flatMap((x) => curr.map((y) => [...x, y])),
    [[]],
  )
}

function createTestMatrix<T extends Record<string, any[]>>(
  optionsMap: T,
): {
  [K in keyof T]: T[K] extends (infer U)[] ? U : never
}[] {
  const keys = Object.keys(optionsMap)
  const values = Object.values(optionsMap)

  return cartesianProduct(values).map<{
    [K in keyof T]: T[K] extends (infer U)[] ? U : never
  }>(
    (combination) =>
      Object.fromEntries(keys.map((key, i) => [key, combination[i]])) as {
        [K in keyof T]: T[K] extends (infer U)[] ? U : never
      },
  )
}

globalThis.createTestMatrix = createTestMatrix

declare global {
  function createTestMatrix<T extends Record<string, any[]>>(
    optionsMap: T,
  ): {
    [K in keyof T]: T[K] extends (infer U)[] ? U : never
  }[]
}

export {}
