import { Flex, Text, VStack } from '@devup-ui/react'
import SyntaxHighlighter from 'react-syntax-highlighter'
import Light from 'react-syntax-highlighter/dist/esm/styles/hljs/atom-one-dark-reasonable'

const CODE = `import { css, Flex, Center } from "@devup-ui/react";
function App() {
  return (
    <div className={css({ flexDir: "row", p: 4 })}>
      {/* Support for many styles */}
      <Flex mt={4}>
        <img src="https://via.placeholder.com/150" alt="avatar" />
      </Flex>
      <Center mt={4} className={css({ mt: "4", fontSize: "xl", fontWeight: "semibold" })}>
        John Doe
      </Center>
    </div>
  );
}`

export function CodeBoard() {
  return (
    <>
      <VStack
        alignItems="center"
        bg="$codeBg"
        borderRadius="20px"
        gap="20px"
        p="20px 30px"
      >
        <Flex p="20px" w="100%">
          <Text color="#FFF" flex="1" overflowX="auto" typography="code">
            <SyntaxHighlighter
              customStyle={{
                backgroundColor: 'transparent',
                width: '100%',
              }}
              language="javascript"
              style={Light}
            >
              {CODE}
            </SyntaxHighlighter>
          </Text>
        </Flex>
      </VStack>
    </>
  )
}
