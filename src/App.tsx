import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/tauri";
import "./App.css";
import {
    Alert,
    AlertIcon,
    Button,
    Card,
    Container,
    Flex,
    Grid,
    GridItem,
    HStack,
    Heading,
    Input,
    Progress,
    SimpleGrid,
    Spacer,
    Stack,
    Text,
    VStack,
    useColorMode,
} from "@chakra-ui/react";
import { DownloadIcon, MoonIcon, SearchIcon, SunIcon } from "@chakra-ui/icons";
import { listen } from "@tauri-apps/api/event";
import { Product } from "./interface";

function App() {
    const { colorMode, toggleColorMode } = useColorMode();
    const [productUrls, setProductUrls] = useState<Product[]>([]);
    const [currentDLFile, setCurrentDLFile] = useState("");
    const [currentProgress, setCurrentProgress] = useState("");

    useEffect(() => {
        listen("DOWNLOAD_PROGRESS", (e) => {
            let data = e.payload as typeof Progress;

            // parse data to json and use Product interface
            let body = JSON.stringify(data);
            var parsed = JSON.parse(body);

            setCurrentProgress(parsed.percentage);

            console.log(data);
        });

        listen("DOWNLOAD_FINISHED", () => {
            setCurrentProgress("100");
            console.log("Finished Downloading!");
        });

        const fetchData = async () =>
            setProductUrls(await invoke("get_products", { page: "1" }));

        fetchData().catch(console.error);
    }, []);

    async function download_product(url: string) {
        await invoke("download_file", { url });
    }

    return (
        <Grid
            templateAreas={{
                base: `"nav" "main"`,
                lg: `"nav nav" "aside main"`,
            }}
            templateColumns={{
                base: "1fr",
                lg: "200px 1fr",
            }}
            borderColor={"transparent"}
        >
            <GridItem area="nav">
                <Flex
                    data-tauri-drag-region
                    as="header"
                    position="fixed"
                    w="100%"
                    alignContent={"space-between"}
                    p={4}
                >
                    <Container data-tauri-drag-region />
                    <Stack direction={"row"} mr={2}>
                        <Button onClick={toggleColorMode}>
                            Switch to{" "}
                            {colorMode === "light" ? (
                                <>
                                    <Text mx={1}>Dark</Text>
                                    <MoonIcon />
                                </>
                            ) : (
                                <>
                                    <Text mx={1}> Light</Text>
                                    <SunIcon />
                                </>
                            )}
                        </Button>
                    </Stack>
                </Flex>
            </GridItem>
            <GridItem area="main">
                <VStack
                    align={"start"}
                    paddingLeft={4}
                    paddingTop={"60px"}
                    marginBottom={5}
                    mr={2}
                >
                    <Alert status="info">
                        <AlertIcon />
                        Downloading {currentDLFile} | {currentProgress}%
                    </Alert>
                    <Flex w={"full"} alignContent={"space-between"} mt={2}>
                        <Heading size={"lg"}>Discover</Heading>
                        <Spacer />
                        <HStack mr={4}>
                            <Input
                                placeholder="Search for softwares"
                                fontSize={"md"}
                            />
                            <Button>
                                <SearchIcon />
                            </Button>
                        </HStack>
                    </Flex>
                    <SimpleGrid
                        columns={{ sm: 1, md: 2, lg: 3, xl: 5 }}
                        mr={2}
                        padding="10px"
                        spacing={6}
                    >
                        {productUrls.map((s) => (
                            <Card>
                                <HStack m={2} padding={2}>
                                    <Text>{s.name}</Text>
                                    <Spacer />
                                    <Button
                                        onClick={() => {
                                            download_product(s.download_link);
                                            setCurrentDLFile(s.name);
                                        }}
                                    >
                                        <DownloadIcon cursor="grab" />
                                    </Button>
                                </HStack>
                            </Card>
                        ))}
                    </SimpleGrid>
                </VStack>
            </GridItem>
        </Grid>
    );
}

export default App;
