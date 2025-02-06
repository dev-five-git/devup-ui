import { glob, readFile, writeFile } from 'node:fs/promises'

const files = await glob('src/**/*.mdx')

const q = []
for await (const file of files) {
  q.push(
    readFile(file).then((content) => {
      const titleIndex = content.toString().indexOf('#')
      return {
        text: content.toString().substring(titleIndex),
        title: /# (.*)/.exec(content.toString())[1],
        url:
          '/devup-ui/' +
          file
            .replace(/src[/\\]app[/\\]\(detail\)[/\\]/, '')
            .replace('page.mdx', ''),
      }
    }),
  )
}

const res = await Promise.all(q)

await writeFile('public/search.json', JSON.stringify(res))
