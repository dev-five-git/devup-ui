import { css, Flex, Text, VStack } from '@devup-ui/react'

import results from '../../../../benchmark-results.json'
import { BenchBox } from './BenchBox'
import { DevupUICard } from './DevupUICard'
import { OtherCard } from './OtherCard'

// The numbers come from the checked benchmark-results.json, which CI rewrites
// after every merge to main (benchmark-refresh.js), so the page shows the same
// run as the READMEs. e2e screenshots hide every `data-benchmark` element.
const WEBPACK = new Map(
  results.rows
    .filter((row) => row.table === 'webpack')
    .map((row) => [row.target, row]),
)

function row(target: string) {
  const found = WEBPACK.get(target)
  if (!found)
    throw new Error(`benchmark-results.json has no webpack ${target} row`)
  return found
}

const seconds = (value: number) => `${value.toFixed(2)}s`
const megabytes = (value: number) => `${(value / 1024 / 1024).toFixed(2)}MB`

const OTHERS = [
  { title: 'Chakra UI', target: 'chakra-ui', url: 'https://chakra-ui.com' },
  { title: 'Mui', target: 'mui', url: 'https://mui.com' },
  { title: 'Kuma UI', target: 'kuma-ui', url: 'https://kuma-ui.com' },
  { title: 'Tailwindcss', target: 'tailwind', url: 'https://tailwindcss.com' },
  { title: 'panda CSS', target: 'panda-css', url: 'https://panda-css.com' },
  { title: 'styleX', target: 'stylex', url: 'https://stylexjs.com' },
  {
    title: 'vanilla extract',
    target: 'vanilla-extract',
    url: 'https://vanilla-extract.style',
  },
].map(({ target, ...card }) => ({ ...card, row: row(target) }))

const OTHER_CARDS = OTHERS.map(({ row, ...card }) => ({
  ...card,
  version: row.version,
  buildTime: seconds(row.median),
  buildSize: megabytes(row.bytes),
}))

const devup = row('devup-ui-single')
const DEVUP_CARD = {
  version: results.run.devupUi,
  buildTime: seconds(devup.median),
  buildSize: megabytes(devup.bytes),
  fastest: OTHERS.every(({ row }) => devup.median <= row.median),
  smallest: OTHERS.every(({ row }) => devup.bytes <= row.bytes),
}

const link = css({ color: 'inherit' })

export function Bench() {
  return (
    <VStack
      alignItems="center"
      overflow="hidden"
      py={['40px', null, '50px', null, '60px']}
      w="100%"
    >
      <VStack
        gap="30px"
        maxW="1440px"
        px={[null, null, null, null, '40px']}
        w="100%"
      >
        <VStack
          gap="16px"
          mx={[4, null, '40px', null, 0]}
          textAlign={['center', null, 'left']}
          wordBreak="keep-all"
        >
          <Text as="h2" color="$title" m="0" typography="h4">
            Comparison Benchmarks
          </Text>
          <Text color="$text" data-benchmark="run" typography="textL">
            Next.js {results.run.next} cold build, median of {results.samples}{' '}
            (GitHub Actions{' '}
            <a className={link} href={results.run.url}>
              run {results.run.id}
            </a>
            , commit{' '}
            <a
              className={link}
              href={`https://github.com/dev-five-git/devup-ui/commit/${results.run.sha}`}
            >
              {results.run.commit}
            </a>
            )
          </Text>
        </VStack>

        <Flex display={[null, null, null, null, 'none']}>
          <DevupUICard {...DEVUP_CARD} />
        </Flex>
        <BenchBox>
          <Flex
            alignItems="flex-end"
            flexWrap={[null, null, null, null, 'wrap']}
            gap={[3, null, 5]}
            justifyContent={[null, null, null, null, 'center']}
            px={[4, null, '40px', null, 0]}
            w="fit-content"
          >
            <Flex display={['none', null, null, null, 'flex']}>
              <DevupUICard {...DEVUP_CARD} />
            </Flex>
            {OTHER_CARDS.map((item) => (
              <OtherCard key={item.title} {...item} />
            ))}
          </Flex>
        </BenchBox>
      </VStack>
    </VStack>
  )
}
