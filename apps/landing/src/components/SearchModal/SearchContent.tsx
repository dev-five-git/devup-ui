'use client'
import { Box, Center, css, Flex, Text, VStack } from '@devup-ui/react'
import Link from 'next/link'
import { Fragment, useEffect, useState } from 'react'

import { useQueryParam } from '@/utils/use-query-param'

interface SearchResult {
  title: string
  text: string
  url: string
}

function getHighlightedParts(text: string, query: string) {
  const parts: { highlighted: boolean; start: number; text: string }[] = []
  const escapedQuery = query.replace(/[.*+?^${}()|[\]\\]/gu, '\\$&')
  const matches = text.matchAll(new RegExp(escapedQuery, 'giu'))
  let start = 0

  for (const match of matches) {
    const matchStart = match.index
    if (matchStart > start) {
      parts.push({
        highlighted: false,
        start,
        text: text.slice(start, matchStart),
      })
    }

    const matchEnd = matchStart + match[0].length
    parts.push({
      highlighted: true,
      start: matchStart,
      text: text.slice(matchStart, matchEnd),
    })
    start = matchEnd
  }

  if (start < text.length || parts.length === 0) {
    parts.push({ highlighted: false, start, text: text.slice(start) })
  }

  return parts
}

export function SearchContent() {
  const query = useQueryParam('query')
  const [data, setData] = useState<SearchResult[]>()
  useEffect(() => {
    if (!query) return

    const controller = new AbortController()
    const normalizedQuery = query.toLowerCase()

    void fetch('/search.json', { signal: controller.signal })
      .then((response) => {
        if (!response.ok) {
          throw new Error(`Search index request failed with ${response.status}`)
        }
        return response.json() as Promise<SearchResult[]>
      })
      .then((results) => {
        setData(
          results.filter(
            (item) =>
              item.title.toLowerCase().includes(normalizedQuery) ||
              item.text.toLowerCase().includes(normalizedQuery),
          ),
        )
      })
      .catch(() => {
        if (!controller.signal.aborted) setData([])
      })

    return () => controller.abort()
  }, [query])
  if (!query) return
  const inner = data ? (
    <>
      {data.length ? (
        data.map((item, idx) => (
          <Fragment key={item.url}>
            <Link
              className={css({ textDecoration: 'none', color: '$text' })}
              href={item.url}
            >
              <VStack
                _hover={{
                  bg: '$menuHover',
                }}
                borderRadius="6px"
                gap="4px"
                justifyContent="center"
                p="10px"
              >
                <Text typography="textSbold">{item.title}</Text>
                <Text color="$caption" typography="caption">
                  {getHighlightedParts(item.text.substring(0, 100), query).map(
                    (part) =>
                      part.highlighted ? (
                        <Text
                          key={part.start}
                          color="$search"
                          fontWeight="bold"
                        >
                          {part.text}
                        </Text>
                      ) : (
                        <Text key={part.start} as="span">
                          {part.text}
                        </Text>
                      ),
                  )}
                  ...
                </Text>
              </VStack>
            </Link>

            {idx !== data.length - 1 && <Box bg="$border" minH="1px" />}
          </Fragment>
        ))
      ) : (
        <Center gap="10px" py="40px">
          <Text
            color="$caption"
            flex="1"
            textAlign="center"
            typography="caption"
          >
            No recent searches
          </Text>
        </Center>
      )}
    </>
  ) : (
    <h3>Loading...</h3>
  )

  return (
    <>
      <Box bg="$border" minH="1px" w="100%" />
      <Flex flex="1" gap="10px" overflowY="auto">
        <VStack gap="10px" w="100%">
          {inner}
        </VStack>
      </Flex>
    </>
  )
}
