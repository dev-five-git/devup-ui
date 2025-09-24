import { readFile, writeFile } from 'node:fs/promises'

import { glob } from 'glob'

const files = await glob('src/**/*.mdx')

const q = []
for await (const file of files) {
  q.push(
    readFile(file, {
      encoding: 'utf-8',
    }).then((content) => {
      const titleIndex = content.toString().indexOf('#')
      if (content.trim().length === 0 || titleIndex === -1) {
        return null
      }
      const fileName = file.split(/[/\\]/).pop()

      return {
        text: content.toString().substring(titleIndex),
        title: /(#)+ (.*)/.exec(content.toString())[1],
        url: file
          .replace(/src[/\\]app[/\\]\(detail\)[/\\]/, '')
          .replace(fileName, ''),
      }
    }),
  )
}

const res = await Promise.all(q)

await writeFile('public/search.json', JSON.stringify(res))
