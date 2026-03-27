'use client'

import { css, Flex, Grid, Text } from '@devup-ui/react'
import Link from 'next/dist/client/link'
import { useMemo, useState } from 'react'

import { ShowcaseCard } from '@/components/showcase/ShowcaseCard'

const SEGMENT_LIST = ['All', 'Website', 'Mobile'] as const
type SegmentType = (typeof SEGMENT_LIST)[number]

interface ShowcaseItem {
  id: string
  name: string
  image: string
  type: string
  link: string
}

interface ShowcaseContentProps {
  list: ShowcaseItem[]
}

export function ShowcaseContent({ list }: ShowcaseContentProps) {
  const [segment, setSegment] = useState<SegmentType>('All')

  const filteredList = useMemo(
    () =>
      segment === 'All'
        ? list
        : list.filter((item) => item.type === segment.toLowerCase()),
    [segment, list],
  )

  return (
    <>
      <Flex
        alignItems="center"
        bg="$menuHover"
        borderRadius="50px"
        p="6px"
        w="fit-content"
      >
        {SEGMENT_LIST.map((item) => (
          <Flex
            key={item}
            alignItems="center"
            as="button"
            bg={segment === item ? '$containerBackground' : 'transparent'}
            border="none"
            borderRadius="999px"
            boxShadow={segment === item ? '0 0 4px 0 #5A44FF33' : 'none'}
            cursor="pointer"
            justifyContent="center"
            onClick={() => setSegment(item)}
            px="30px"
            py="10px"
            transition="all 0.2s"
          >
            <Text
              color={segment === item ? '$primary' : '$caption'}
              typography="buttonM"
            >
              {item}
            </Text>
          </Flex>
        ))}
      </Flex>
      <Grid
        columnGap="24px"
        gridTemplateColumns={[null, 'repeat(3, 1fr)']}
        rowGap="36px"
      >
        {filteredList.map((showcase) => (
          <Link
            key={showcase.id}
            className={css({ textDecoration: 'none' })}
            href={showcase.link}
            rel="noopener"
            target="_blank"
          >
            <ShowcaseCard {...showcase} />
          </Link>
        ))}
      </Grid>
    </>
  )
}
