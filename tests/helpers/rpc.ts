import * as anchor from "@coral-xyz/anchor";

export const provider = anchor.AnchorProvider.env();
anchor.setProvider(provider);
export const connection = provider.connection;

export async function delayForMainnet() {
  if (connection.rpcEndpoint.includes("mainnet")) {
    setTimeout(() => {}, 5000); // 5 seconds delay
  }
}