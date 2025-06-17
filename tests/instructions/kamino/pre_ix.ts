import { KAMINO_FARM_MAINNET, KAMINO_LENDING, KAMINO_LENDING_MAIN_MARKET, KAMINO_LENDING_MAIN_MARKET_AUTHORITY, KAMINO_SCOPE_PRICES } from "../../helpers/const";
import { User } from "../../objects/user";
import { provider } from "../../helpers/setup";
import { Program } from "@coral-xyz/anchor";
import { KaminoLending } from "../../idl/kamino_lending";
import kamino_idl from "../../idl/kamino_lending.json";
import { PublicKey, TransactionInstruction } from "@solana/web3.js";
import * as anchor from "@coral-xyz/anchor";
import { Token } from "../../objects/token";

const kaminoProgram = new Program<KaminoLending>(kamino_idl, provider);
const kaminoProgramId = kaminoProgram.programId;

export async function getPreInstructions(user: User, token: Token, reserves: PublicKey[], reserves_obligation: PublicKey[]): Promise<TransactionInstruction[]> {    

  const ixRefreshReserve: TransactionInstruction[] = await refreshReserve(token, reserves);

    // RefreshObligation, Add the reserve accounts in remaining accounts (if any)
  const txRefreshObligation: TransactionInstruction = await refreshObligation(user, reserves_obligation);
  
  ixRefreshReserve.push(txRefreshObligation);

    // RefreshObligationFarmsForReserve
    // const IxRefreshObligationFarmsForReserve = refreshObligationFarmsForReserve(user, token, mode); 
    
  return ixRefreshReserve;
}

async function refreshReserve(token: Token, reserves: PublicKey[]): Promise<TransactionInstruction[]> {
  let ixs: TransactionInstruction[] = [];
  for (const reserve of reserves) {
    const ix = await kaminoProgram.methods.refreshReserve()
      .accounts({
        reserve: reserve,
        lendingMarket: KAMINO_LENDING_MAIN_MARKET,
        pythOracle: kaminoProgramId,
        switchboardPriceOracle: kaminoProgramId,
        switchboardTwapOracle: kaminoProgramId,
        scopePrices: KAMINO_SCOPE_PRICES,
      })
      .instruction();
    ixs.push(ix);
  }
  return ixs;
}

async function refreshObligation(user: User, reserves: PublicKey[]): Promise<TransactionInstruction> {
  return await kaminoProgram.methods.refreshObligation()
      .accounts({
          lendingMarket: KAMINO_LENDING_MAIN_MARKET,
          obligation: user.obligation,
      })
      .remainingAccounts(
          reserves.map(reserve => ({
              isSigner: false,
              isWritable: true,
              pubkey: reserve
          }))
      )
      .instruction();
} 

async function refreshObligationFarmsForReserve(user: User, token: Token, mode: number): Promise<TransactionInstruction> {
    return await kaminoProgram.methods.refreshObligationFarmsForReserve(mode)
        .accounts({
        crank: user.wallet.publicKey,
        baseAccounts: {
            obligation: user.obligation,
            lendingMarketAuthority: KAMINO_LENDING_MAIN_MARKET_AUTHORITY,
            reserve: token.reserve,
            reserveFarmState: mode === 0 ? token.kaminReserveFarmStateCollateral : token.kaminReserveFarmStateDebt,
            obligationFarmUserState: user.getObligationFarmForToken(token.tokenIndex), // change this
            lendingMarket: KAMINO_LENDING_MAIN_MARKET,
        },
        farmsProgram: KAMINO_FARM_MAINNET,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        systemProgram: anchor.web3.SystemProgram.programId,
        })
      .instruction();
}