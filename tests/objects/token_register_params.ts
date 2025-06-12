import { BN } from "@coral-xyz/anchor";

export interface InterestRateParams {
  util0: number;
  rate0: number;
  util1: number;
  rate1: number;
  maxRate: number;
  adjustmentFactor: number;
}

export interface OracleConfigParams {
  confFilter: number;
  maxStalenessSlots: number | null;
}

export interface TokenRegisterParams {
  // tokenIndex: number;
  // name: string;
  oracleConfig: OracleConfigParams;
  interestRateParams: InterestRateParams;
  loanFeeRate: number;
  loanOriginationFeeRate: number;
  maintAssetWeight: number;
  initAssetWeight: number;
  maintLiabWeight: number;
  initLiabWeight: number;
  liquidationFee: number;
  stablePriceDelayIntervalSeconds: number;
  stablePriceDelayGrowthLimit: number;
  stablePriceGrowthLimit: number;
  minVaultToDepositsRatio: number;
  netBorrowLimitWindowSizeTs: BN;
  netBorrowLimitPerWindowQuote: BN;
  borrowWeightScaleStartQuote: number;
  depositWeightScaleStartQuote: number;
  reduceOnly: number;
  interestCurveScaling: number;
  interestTargetUtilization: number;
  depositLimit: BN;
  zeroUtilRate: number;
  platformLiquidationFee: number;
  disableAssetLiquidation: boolean;
  collateralFeePerDay: number;
  tier: string;
}

export const defaultTokenRegisterParams: TokenRegisterParams = {
  oracleConfig: {
    confFilter: 0.01,
    maxStalenessSlots: 500000000,
  },
  interestRateParams: {
    util0: 0.8,
    rate0: 0.02,
    util1: 0.9,
    rate1: 0.05,
    maxRate: 0.1,
    adjustmentFactor: 0.01,
  },
  loanFeeRate: 0.002,
  loanOriginationFeeRate: 0.001,
  maintAssetWeight: 0.8,
  initAssetWeight: 0.7,
  maintLiabWeight: 1.2,
  initLiabWeight: 1.3,
  liquidationFee: 0.05,
  stablePriceDelayIntervalSeconds: 60,
  stablePriceDelayGrowthLimit: 0.02,
  stablePriceGrowthLimit: 0.03,
  minVaultToDepositsRatio: 0.1,
  netBorrowLimitWindowSizeTs: new BN(600),
  netBorrowLimitPerWindowQuote: new BN(1000000),
  borrowWeightScaleStartQuote: 50000,
  depositWeightScaleStartQuote: 50000,
  reduceOnly: 0,
  interestCurveScaling: 1,
  interestTargetUtilization: 0.8,
  depositLimit: new BN(1000000),
  zeroUtilRate: 0.01,
  platformLiquidationFee: 0.02,
  disableAssetLiquidation: false,
  collateralFeePerDay: 0.001,
  tier: "tier",
};

export const defaultTokenRegisterParamsUSDC: TokenRegisterParams = {
  oracleConfig: {
    confFilter: 0.01,
    maxStalenessSlots: 500000000,
  },
  interestRateParams: {
    util0: 0.8,
    rate0: 0.02,
    util1: 0.9,
    rate1: 0.05,
    maxRate: 0.1,
    adjustmentFactor: 0.01,
  },
  loanFeeRate: 0.002,
  loanOriginationFeeRate: 0.001,
  maintAssetWeight: 0.8,
  initAssetWeight: 0.7,
  maintLiabWeight: 1.2,
  initLiabWeight: 1.3,
  liquidationFee: 0.05,
  stablePriceDelayIntervalSeconds: 60,
  stablePriceDelayGrowthLimit: 0.02,
  stablePriceGrowthLimit: 0.03,
  minVaultToDepositsRatio: 0.1,
  netBorrowLimitWindowSizeTs: new BN(5 * 10 ** 6),
  netBorrowLimitPerWindowQuote: new BN(20 * 10 ** 6),
  borrowWeightScaleStartQuote: 50000,
  depositWeightScaleStartQuote: 50000,
  reduceOnly: 0,
  interestCurveScaling: 1,
  interestTargetUtilization: 0.8,
  depositLimit: new BN(40 * 10 ** 6), 
  zeroUtilRate: 0.01,
  platformLiquidationFee: 0.02,
  disableAssetLiquidation: false,
  collateralFeePerDay: 0.001,
  tier: "tier",
};

export const defaultTokenRegisterParamsPYUSD: TokenRegisterParams = {
  oracleConfig: {
    confFilter: 0.01,
    maxStalenessSlots: 500000000,
  },
  interestRateParams: {
    util0: 0.8,
    rate0: 0.02,
    util1: 0.9,
    rate1: 0.05,
    maxRate: 0.1,
    adjustmentFactor: 0.01,
  },
  loanFeeRate: 0.002,
  loanOriginationFeeRate: 0.001,
  maintAssetWeight: 0.8,
  initAssetWeight: 0.7,
  maintLiabWeight: 1.2,
  initLiabWeight: 1.3,
  liquidationFee: 0.05,
  stablePriceDelayIntervalSeconds: 60,
  stablePriceDelayGrowthLimit: 0.02,
  stablePriceGrowthLimit: 0.03,
  minVaultToDepositsRatio: 0.1,
  netBorrowLimitWindowSizeTs: new BN(5 * 10 ** 6),
  netBorrowLimitPerWindowQuote: new BN(20 * 10 ** 6),
  borrowWeightScaleStartQuote: 50000,
  depositWeightScaleStartQuote: 50000,
  reduceOnly: 0,
  interestCurveScaling: 1,
  interestTargetUtilization: 0.8,
  depositLimit: new BN(40 * 10 ** 6),
  zeroUtilRate: 0.01,
  platformLiquidationFee: 0.02,
  disableAssetLiquidation: false,
  collateralFeePerDay: 0.001,
  tier: "tier",
};