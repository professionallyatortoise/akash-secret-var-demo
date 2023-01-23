const express = require("express");
const dotenv = require("dotenv");
import { Wallet } from "secretjs";

const wallet = new Wallet();
const myAddress = wallet.address;
const myMnemonicPhrase = wallet.mnemonic;

console.log("Server started");
dotenv.config();

const app = express();
const port = 3002;

app.get("/", (req: any, res: any) => {
  res.send(
    `Hello World! My address is ${myAddress}, my mock address is ${process.env.CONTRACT_ADDRESS}}`
  );
});

app.listen(port, () => {
  console.log(`[server]: Server is running at http://localhost:${port}`);
  console.log(`[server]: My address is ${myAddress}`);
});
