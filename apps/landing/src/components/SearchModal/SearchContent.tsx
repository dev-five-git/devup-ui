'use client'
import { Box, Center, css, Flex, Text, VStack } from '@devup-ui/react'
import Link from 'next/link'
import { useSearchParams } from 'next/navigation'
import { Fragment, useEffect, useMemo, useState } from 'react'

import { URL_PREFIX } from '../../constants'

export function SearchContent() {
  const query = useSearchParams().get('query')
  const [data, setData] = useState<
    {
      title: string
      text: string
      url: string
    }[]
  >()
  useEffect(() => {
    if (query) {
      fetch(URL_PREFIX + '/search.json')
        .then((response) => response.json())
        .then(
          (
            data: {
              title: string
              text: string
              url: string
            }[],
          ) => {
            setData(
              data.filter(
                (item) =>
                  item.title.toLowerCase().includes(query.toLowerCase()) ||
                  item.text.toLowerCase().includes(query.toLowerCase()),
              ),
            )
          },
        )
    }
  }, [query])
  const reg = useMemo(() => new RegExp(`(${query})`, 'gi'), [query])
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
                  {item.text
                    .substring(0, 100)
                    .split(reg)
                    .map((part, idx) =>
                      part.toLowerCase() === query.toLowerCase() ? (
                        <Text key={idx} color="$search" fontWeight="bold">
                          {part}
                        </Text>
                      ) : (
                        <Text key={idx} as="span">
                          {part}
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
