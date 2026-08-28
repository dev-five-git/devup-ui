export default {
  plugins: {
    '@stylexswc/postcss-plugin': {
      include: ['app/**/*.{js,jsx,ts,tsx}'],
      rsOptions: {
        dev: process.env.NODE_ENV !== 'production',
      },
    },
  },
}
