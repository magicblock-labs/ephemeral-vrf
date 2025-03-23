import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { UseRandomness } from "../target/types/use_randomness";

describe("use-randomness", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.useRandomness as Program<UseRandomness>;

  it("Request randomness", async () => {
    const randomSeed = Math.floor(Math.random() * 256);
    const tx = await program.methods.requestRandomness(randomSeed).rpc();
    console.log("Request randomness", tx);
  });
});
