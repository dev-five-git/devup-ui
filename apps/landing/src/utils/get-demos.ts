import type { ComponentType } from 'react'

import { DEMOS } from './demos.generated'

export async function getDemos(
  dir: string,
): Promise<[ComponentType, string][]> {
  const prefix = `${dir}/demo/`

  return DEMOS.filter(([label]) => label.startsWith(prefix)).map<
    [ComponentType, string]
  >(([label, component]) => [component, label])
}
