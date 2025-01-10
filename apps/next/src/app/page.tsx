'use client'
import styles from "./page.module.css";
import {Box} from "@devup-ui/react";
import {useState} from "react";

export default function Home() {
  const bg = "red"
  const [color, setColor] = useState("yellow")

  return (
    <div className={styles.page}>
      <Box bg={bg} color={color} fontSize={32}>hello</Box>
      <button onClick={() => setColor("blue")}>
        Change
      </button>


    </div>
  );
}
