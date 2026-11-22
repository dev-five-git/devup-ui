import type { Locale } from 'next-intl'

import en from './messages/en.json'
import ko from './messages/ko.json'

type Messages = typeof en
type MessageOverrides = {
  [Namespace in keyof Messages]?: Partial<Messages[Namespace]>
}

const koreanOverrides: MessageOverrides = ko

/** English supplies every key; Korean replaces the keys it translates. */
function mergeMessages(overrides: MessageOverrides): Messages {
  return Object.fromEntries(
    Object.entries(en).map(([namespace, messages]) => [
      namespace,
      { ...messages, ...overrides[namespace as keyof Messages] },
    ]),
  ) as Messages
}

const koreanMessages = mergeMessages(koreanOverrides)

export function getLocaleMessages(locale: Locale): Messages {
  return locale === 'ko' ? koreanMessages : en
}
