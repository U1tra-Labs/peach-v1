import { Program, Wallet } from "@coral-xyz/anchor";
import * as anchor from "@coral-xyz/anchor";
import NodeWallet from "@coral-xyz/anchor/dist/cjs/nodewallet";
import { AddressLookupTableProgram, Connection, PublicKey, Transaction } from "@solana/web3.js";
import { PeachV1 } from "../../target/types/peach_v1";

export const provider = anchor.AnchorProvider.env();
anchor.setProvider(provider);
export const program = anchor.workspace.PeachV1 as Program<PeachV1>;
export const envProviderPayer = (provider.wallet as NodeWallet).payer;

export async function createLookupTableAddress(connection: Connection, wallet: Wallet) {

    const recentSlot = await connection.getSlot("finalized");

    const [createLutIx, lookupTableAddress] = AddressLookupTableProgram.createLookupTable({
        authority: wallet.publicKey,
        payer: wallet.publicKey,
        recentSlot: recentSlot
    });

    const transaction = new Transaction().add(createLutIx);
    const signature = await provider.connection.sendTransaction(transaction, [wallet.payer]);
    await provider.connection.confirmTransaction(signature);
    
    console.log(`Address Lookup Table: ${lookupTableAddress.toBase58()}`);

    return lookupTableAddress;
}