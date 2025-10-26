import { Center, css, Flex, Grid, Text, VStack } from "@devup-ui/react";
import Link from "next/link";
import Description from "./Description.mdx";

import { Icons } from "@/components/icons/components";
import { COMPONENT_GROUPS } from "@/constants";

import Card from "../Card";

export default function Page() {
  return (
    <VStack gap="16px">
      <Text color="$primary" typography="captionBold">
        Overview
      </Text>
      <Text color="$title" typography="h4">
        Devup UI Components
      </Text>
      <Text color="$text" typography="bodyReg">
        <Description />
      </Text>
      <Text color="$title" typography="h4" pt="30px">
        Examples
      </Text>
      <VStack gap="16px" overflow="visible" pb="30px">
        <Text color="$title" typography="h6">
          Form
        </Text>
        <Grid
          gap={["10px", null, null, null, "20px"]}
          gridTemplateColumns={[
            "repeat(1, 1fr)",
            "repeat(3, 1fr)",
            null,
            "repeat(4, 1fr)",
            "repeat(5, 1fr)",
          ]}
        >
          {COMPONENT_GROUPS.Form.map((component) => {
            const Icon =
              Icons[
                `Icon${component
                  .split("-")
                  .map((item) => item.charAt(0).toUpperCase() + item.slice(1))
                  .join("")}Comp`
              ];
            return (
              <Link
                key={component}
                className={css({ textDecoration: "none" })}
                href={`/components/${component}`}
              >
                <Card>
                  <Center h="140px">
                    <Icon className={css({ w: "100%" })} />
                  </Center>
                  <Flex
                    alignItems="center"
                    borderTop="1px solid $border"
                    gap="10px"
                    px="16px"
                    py="12px"
                  >
                    <Text
                      color="$text"
                      textAlign="right"
                      typography="buttonSmid"
                    >
                      {component
                        .split("-")
                        .map(
                          (item) => item.charAt(0).toUpperCase() + item.slice(1)
                        )
                        .join(" ")}
                    </Text>
                  </Flex>
                </Card>
              </Link>
            );
          })}
        </Grid>
      </VStack>
      <VStack gap="16px" overflow="visible" py="30px">
        <Text color="$title" typography="h6">
          Layout
        </Text>
        <Grid
          gap={["10px", null, null, null, "20px"]}
          gridTemplateColumns={[
            "repeat(1, 1fr)",
            "repeat(3, 1fr)",
            null,
            "repeat(4, 1fr)",
            "repeat(5, 1fr)",
          ]}
          overflow="visible"
        >
          {COMPONENT_GROUPS.Layout.map((component) => {
            const Icon =
              Icons[
                `Icon${component
                  .split("-")
                  .map((item) => item.charAt(0).toUpperCase() + item.slice(1))
                  .join("")}Comp`
              ];
            return (
              <Link
                key={component}
                className={css({ textDecoration: "none" })}
                href={`/components/${component}`}
              >
                <Card>
                  <Center h="140px">
                    <Icon className={css({ w: "100%" })} />
                  </Center>
                  <Flex
                    alignItems="center"
                    borderTop="1px solid $border"
                    gap="10px"
                    px="16px"
                    py="12px"
                  >
                    <Text
                      color="$text"
                      textAlign="right"
                      typography="buttonSmid"
                    >
                      {component
                        .split("-")
                        .map(
                          (item) => item.charAt(0).toUpperCase() + item.slice(1)
                        )
                        .join(" ")}
                    </Text>
                  </Flex>
                </Card>
              </Link>
            );
          })}
        </Grid>
      </VStack>
      <VStack gap="16px" overflow="visible" py="30px">
        <Text color="$title" typography="h6">
          Theme
        </Text>
        <Grid
          gap={["10px", null, null, null, "20px"]}
          gridTemplateColumns={[
            "repeat(1, 1fr)",
            "repeat(3, 1fr)",
            null,
            "repeat(4, 1fr)",
            "repeat(5, 1fr)",
          ]}
          overflow="visible"
        >
          {COMPONENT_GROUPS.Theme.map((component) => {
            const Icon =
              Icons[
                `Icon${component
                  .split("-")
                  .map((item) => item.charAt(0).toUpperCase() + item.slice(1))
                  .join("")}Comp`
              ];
            return (
              <Link
                key={component}
                className={css({ textDecoration: "none" })}
                href={`/components/${component}`}
              >
                <Card>
                  <Center h="140px">
                    <Icon className={css({ w: "100%" })} />
                  </Center>
                  <Flex
                    alignItems="center"
                    borderTop="1px solid $border"
                    gap="10px"
                    px="16px"
                    py="12px"
                  >
                    <Text
                      color="$text"
                      textAlign="right"
                      typography="buttonSmid"
                    >
                      {component
                        .split("-")
                        .map(
                          (item) => item.charAt(0).toUpperCase() + item.slice(1)
                        )
                        .join(" ")}
                    </Text>
                  </Flex>
                </Card>
              </Link>
            );
          })}
        </Grid>
      </VStack>
    </VStack>
  );
}
