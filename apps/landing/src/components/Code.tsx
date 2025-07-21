import { Box } from '@devup-ui/react'
import SyntaxHighlighter from 'react-syntax-highlighter'
import Dark from 'react-syntax-highlighter/dist/esm/styles/hljs/atom-one-dark'
import Light from 'react-syntax-highlighter/dist/esm/styles/hljs/atom-one-light'

export const Code = ({
  language,
  value,
}: {
  language: string
  value: string
}) => {
  return (
    <>
      <Box
        _themeDark={{
          display: 'none',
        }}
      >
        <SyntaxHighlighter language={language} showLineNumbers style={Light}>
          {value}
        </SyntaxHighlighter>
      </Box>
      <Box
        _themeDark={{
          display: 'block',
        }}
        display="none"
      >
        <SyntaxHighlighter language={language} showLineNumbers style={Dark}>
          {value}
        </SyntaxHighlighter>
      </Box>
    </>
  )
}
