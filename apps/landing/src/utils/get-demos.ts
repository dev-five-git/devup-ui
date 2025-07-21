import { existsSync } from 'node:fs'
import { readdir } from 'node:fs/promises'
import { join, relative } from 'node:path'

export async function getDemos(
  dir: string,
): Promise<[React.ComponentType, string][]> {
  const dirPath = join(
    process.cwd(),
    'src',
    'app',
    '(detail)',
    'components',
    '[component]',
    dir,
    'demo',
  )
  if (!existsSync(dirPath)) {
    return []
  }
  const demos = await readdir(dirPath)

  return Promise.all(
    demos.map<Promise<[React.ComponentType, string]>>((item) =>
      import(
        './../' +
          relative(process.cwd(), `${dirPath}/${item}`)
            .replace('.tsx', '')
            .replaceAll('\\', '/')
            .slice(4)
      ).then((m) => [m.default, `${dir}/demo/${item}`]),
    ),
  )
}
