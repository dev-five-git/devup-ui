export default {
  '**/*.{js,jsx,ts,tsx,mjs,md,mdx,json,json5,jsonc}': (files) => {
    return [
      `npx eslint --config eslint.config.mjs --fix ${files.join(' ')}`,
      `git add ${files.join(' ')}`,
    ]
  },
}
