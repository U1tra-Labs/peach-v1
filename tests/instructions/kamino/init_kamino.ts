import { PublicKey, SystemProgram } from "@solana/web3.js";
import { InitObligationArgs, User } from "../../objects/user";
import { KAMINO_FARM_MAINNET, KAMINO_LENDING, KAMINO_LENDING_MAIN_MARKET, KAMINO_LENDING_MAIN_MARKET_AUTHORITY, SEED1_Account } from "../../helpers/const";
import { program } from "../../helpers/setup";
import { createLookupTableAddress } from "../../helpers/create_alt";
import { Token } from "../../objects/token";
import * as anchor from "@coral-xyz/anchor";

export async function kaminoInitUserMetadata(user: User, market: PublicKey) {
    const lookupTableAddress = await createLookupTableAddress(program.provider.connection, user.wallet);
        
    const signature = await program.methods.kaminoInitUserMetadata(lookupTableAddress)
        .accounts({
        owner: user.wallet.publicKey,
        market: market,
        peachAccount: user.peachAccount,
        userMetadata: user.userMetadata,
        referrerUserMetadata: KAMINO_LENDING,
        systemProgram: SystemProgram.programId,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY
        })
        .signers([user.wallet])
        .rpc();

    console.log("Init user metadata transaction signature: ", signature);

}

export async function kaminoInitObligation(user: User, market: PublicKey, args: InitObligationArgs) {
    const signature = await program.methods.kaminoInitObligation(args)
        .accounts({
        market: market,
        peachAccount: user.peachAccount,
        owner: user.wallet.publicKey,
        obligation: user.obligation,
        lendingMarket: KAMINO_LENDING_MAIN_MARKET,
        seedOneAccount: SEED1_Account,
        seedTwoAccount: SEED1_Account,
        ownerUserMetadata: user.userMetadata,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        systemProgram: SystemProgram.programId
        })
        .signers([user.wallet])
        .rpc();

    console.log("Init Obligation transaction signature: ", signature);
}

export async function kaminoInitObligationFarms(user: User, token: Token, market: PublicKey, mode: number) {
    
    const signature = await program.methods.kaminoInitObligationFarmForReserve(mode)
        .accounts({
        market: market,
        peachAccount: user.peachAccount,
        owner: user.wallet.publicKey,
        obligation: user.obligation,
        lendingMarketAuthority: KAMINO_LENDING_MAIN_MARKET_AUTHORITY,
        reserve: token.reserve,
        reserveFarmState: token.kaminReserveFarmState,
        obligationFarm: user.getObligationFarmForToken(token.tokenIndex),
        lendingMarket: KAMINO_LENDING_MAIN_MARKET,
        farmsProgram: KAMINO_FARM_MAINNET,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        systemProgram: SystemProgram.programId,
        })
        .signers([user.wallet])
        .rpc();
    
    console.log("Init Obligation Farms transaction signature: ", signature);
}