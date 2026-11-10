import { writeFileSync } from 'node:fs'

// support mjs config
writeFileSync('pkg/package.json', JSON.stringify({}), 'utf8')
