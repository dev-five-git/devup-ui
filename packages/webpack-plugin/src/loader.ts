import {codeExtract} from "@devup-ui/wasm";
import type {RawLoaderDefinitionFunction} from "webpack";


const devupUiLoader: RawLoaderDefinitionFunction = function (
  source,
) {
  // tell Webpack this loader is async
  const callback = this.async();
  const id = this.resourcePath;
  if (
    id.includes("/node_modules/") ||
    id.includes("@devup-ui/react") ||
    !/\.[tj](s|sx)?$/.test(id)
  ) {
    callback(null, source);
    return;
  }
  const output = codeExtract(this.resourcePath, source.toString(), "@devup-ui/react")

  const code = output.code()
  callback(null, code);
  return;
};
export default devupUiLoader
