module.exports = {
  plugins: {
    '@stylexjs/postcss-plugin': {
      include: ['app/**/*.{js,jsx,ts,tsx}'],
      exclude: ['**/node_modules/**', '**/.next/**'],
    },
  },
}
