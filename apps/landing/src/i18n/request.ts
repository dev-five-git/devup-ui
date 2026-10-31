import { getRequestConfig } from 'next-intl/server'

export default getRequestConfig(async ({ requestLocale }) => ({
  locale: (await requestLocale) ?? 'en',
  timeZone: 'UTC',
}))
