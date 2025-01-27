import SyntaxHighlighter from 'react-syntax-highlighter'
import Light from 'react-syntax-highlighter/dist/esm/styles/hljs/atom-one-dark-reasonable'

export const Code = ({
  language,
  value,
}: {
  language: string
  value: string
}) => {
  return (
    <SyntaxHighlighter language={language} style={Light}>
      {value}
    </SyntaxHighlighter>
  )
}
