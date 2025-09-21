import * as stylex from "@stylexjs/stylex";

import ThemeableButton from "../app/components/ThemeableButton";
import renderer from "react-test-renderer";
import React from "react";

describe("ThemeableButton", () => {
  it("renders default state", () => {
    const btn = renderer
      .create(<ThemeableButton onClick={() => {}}>ClickMe</ThemeableButton>)
      .toJSON();

    expect(btn).toMatchInlineSnapshot(`
     <button
       className="ThemeableButton__styles.base"
       onClick={[Function]}
     >
       ClickMe
     </button>
    `);
  });
});
