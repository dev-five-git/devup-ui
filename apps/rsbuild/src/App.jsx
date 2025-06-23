import { Box } from '@devup-ui/react';

const App = () => {
  return (
    <div className="content">
      <Box bg="blue" _hover={{ bg: 'red' }} color="white">
        Rsbuild support
      </Box>
    </div>
  );
};

export default App;
