import { Box, Flex, Image, Text, VStack } from '@devup-ui/react'
import Link from 'next/link'
import { cache } from 'react'

import { Github } from '../../../components/Header/Github'
import { Insta } from '../../../components/Header/Insta'

interface TeamCardProps {
  userId: string
  role: string
  instaId?: string
}

interface GithubUser {
  avatar_url: string
  name: string | null
}

const getGithubUser = cache(async (userId: string) => {
  const response = await fetch(`https://api.github.com/users/${userId}`)
  if (!response.ok) {
    throw new Error(`GitHub user request failed with ${response.status}`)
  }
  return response.json() as Promise<GithubUser>
})

export async function TeamCard({ userId, role, instaId }: TeamCardProps) {
  const data = await getGithubUser(userId)
  const avatarUrl = data.avatar_url
  const name = data.name ?? userId
  return (
    <VStack
      alignItems="flex-end"
      bg="$background"
      borderRadius="16px"
      boxShadow="0px 0px 6px 0px rgba(0, 0, 0, 0.18)"
      flex="1"
      justifyContent="space-between"
      overflow="hidden"
      pb="24px"
      pos="relative"
    >
      <Box bg="#eccafa" h="64px" pos="absolute" w="100%" />
      <Flex
        justifyContent="space-between"
        mt="24px"
        px="24px"
        w="100%"
        zIndex={1}
      >
        <VStack gap="12px">
          <Image
            alt={`${name}'s GitHub avatar`}
            bg="#eccafa"
            borderRadius="100%"
            boxSize="80px"
            src={avatarUrl}
          />
          <VStack gap="4px">
            <Text color="$title" typography="buttonLbold">
              {name}
            </Text>
            <Text color="$caption" typography="caption">
              {role}
            </Text>
          </VStack>
        </VStack>
        <Flex alignItems="end" gap="10px">
          <Link href={`https://github.com/${userId}`} target="_blank">
            <Github />
          </Link>
          {instaId && (
            <Link href={`https://instagram.com/${instaId}`} target="_blank">
              <Insta />
            </Link>
          )}
        </Flex>
      </Flex>
    </VStack>
  )
}
