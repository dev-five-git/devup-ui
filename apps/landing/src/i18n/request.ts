import { getRequestConfig } from 'next-intl/server'

import { getLocaleMessages } from './messages'

export default getRequestConfig(async ({ requestLocale }) => {
  const locale = (await requestLocale) === 'ko' ? 'ko' : 'en'
  return {
    locale,
    messages: getLocaleMessages(locale),
    timeZone: 'UTC',
  }
})
