import { Box, Flex, Text } from '@devup-ui/react';

const App = () => {
  return (
    <div className="content">
      <Box bg="blue" _hover={{ bg: 'red' }} color="white">
        Rsbuild support
      </Box>
      <Flex>
        <Text color="#777">a</Text>
        <Text color="#777">b</Text>
        <Text typography="header">typo</Text>
      </Flex>
    </div>
  );
};

export default App;

