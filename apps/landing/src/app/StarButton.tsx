'use client'

import './StarButton.css'

import { Center, css, Flex, Image, Text } from '@devup-ui/react'
import Link from 'next/link'
import { useEffect, useState } from 'react'

export default function StarButton() {
  const [starCount, setStarCount] = useState<number | null>(null)

  useEffect(() => {
    const abortController = new AbortController()
    const fetchStarCount = async () => {
      try {
        const data = await fetch(
          'https://api.github.com/repos/dev-five-git/devup-ui',
          {
            signal: abortController.signal,
          },
        ).then((res) => res.json())
        setStarCount(data.stargazers_count)
      } catch (error) {
        console.error(error)
      } finally {
        setStarCount((prev) => (typeof prev === 'number' ? prev : -1))
      }
    }
    fetchStarCount()

    return () => {
      abortController.abort()
    }
  }, [])

  return (
    <Link
      className={css({
        textDecoration: 'none',
      })}
      href="https://github.com/dev-five-git/devup-ui"
      target="_blank"
    >
      <Flex
        _active={{
          bg: '$menuActive',
        }}
        _hover={{
          bg: '$menuHover',
        }}
        alignItems="center"
        bg="$containerBackground"
        border="1px solid $imageBorder"
        borderRadius="12px"
        cursor="pointer"
        h="100%"
        role="group"
        transition="all 0.1s ease-in-out"
      >
        <Flex
          alignItems="center"
          borderRadius="12px 0 0 12px"
          gap="10px"
          pl="16px"
          pr="20px"
          py="10px"
        >
          <Image
            _groupHover={{
              transform: 'scale(1.1)',
            }}
            aspectRatio="1"
            boxSize="24px"
            src="/icons/solar_star-bold.svg"
            transition="transform 0.2s ease-in-out"
          />
          <Text color="$text" textAlign="center" typography="buttonLsemiB">
            Star
          </Text>
        </Flex>
        <Center
          bg="$starBg"
          borderLeft="1px solid $imageBorder"
          borderRadius="0 12px 12px 0"
          h="100%"
          px="16px"
        >
          {starCount === null ? (
            <Image
              alt="Loading"
              animation="spin 1s linear infinite"
              boxSize="20px"
              src="/spinner.svg"
            />
          ) : (
            <Text color="$primary" textAlign="center" typography="buttonLsemiB">
              {starCount}
            </Text>
          )}
        </Center>
      </Flex>
    </Link>
  )
}
