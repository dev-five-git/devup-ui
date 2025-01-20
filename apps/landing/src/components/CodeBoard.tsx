import { Box, Flex, Text, VStack } from '@devup-ui/react'
import SyntaxHighlighter from 'react-syntax-highlighter'
import { docco } from 'react-syntax-highlighter/dist/esm/styles/hljs'

const CODE = `import { css } from "./styled-system/css";
function App() {
  return (
    <div className={stack({ direction: "row", p: "4" })}>
      <div className={circle({ size: "5rem", overflow: "hidden" })}>
        <img src="https://via.placeholder.com/150" alt="avatar" />
      </div>
      <div className={css({ mt: "4", fontSize: "xl", fontWeight: "semibold" })}>
        John Doe
      </div>
      <div className={css({ mt: "2", fontSize: "sm", color: "gray.600" })}>
        john@doe.com
      </div>
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
          <Text color="#FFF" flex="1" typography="code">
            <SyntaxHighlighter
              customStyle={{
                backgroundColor: 'transparent',
                width: '100%',
              }}
              language="javascript"
              style={docco}
            >
              {CODE}
            </SyntaxHighlighter>
          </Text>
        </Flex>
        <Flex
          alignItems="center"
          bg="$secondary"
          borderRadius="1000px"
          gap="10px"
          p="6px"
          w="520px"
        >
          <Box
            bg="$containerBackground"
            borderRadius="100px"
            boxShadow="0px 0px 8px 0px rgba(0, 0, 0, 0.25)"
            h="46px"
            style={{ left: '6px', top: '6px', position: 'absolute' }}
            w="162.67px"
          />
          <Flex
            alignItems="center"
            borderRadius="1000px"
            flex="1"
            p="12px 34px"
          >
            <Text typography="buttonM">code menu</Text>
          </Flex>
          <Flex
            alignItems="center"
            borderRadius="1000px"
            flex="1"
            p="12px 34px"
          >
            <Text opacity="0.6" typography="buttonM">
              code menu
            </Text>
          </Flex>
          <Flex
            alignItems="center"
            borderRadius="1000px"
            flex="1"
            p="12px 34px"
          >
            <Text opacity="0.6" typography="buttonM">
              code menu
            </Text>
          </Flex>
        </Flex>
      </VStack>
    </>
  )
}
