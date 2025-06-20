# Peach V1 Testing Mechanism Overview

This document describes **what** is being tested and **how the test suite is structured**, reflecting the latest code, Kamino integration, and best practices for robust protocol testing.

---

## 1. **Test Suite Structure**

The Peach V1 test suite is organized into several logical sections, each targeting a specific area of protocol functionality:

- **Account Lifecycle:**  
  Tests for account creation, registration, deregistration, and closure.
- **Native Lending Operations:**  
  Tests for core lending/borrowing flows using the native Peach protocol.
- **Kamino Integration:**  
  Tests for Kamino lending/borrowing flows, cross-protocol interactions, and Kamino-specific account/farm logic.
- **Collateral and Fee Logic:**  
  Tests for collateral management, farm state, and fee charging.
- **Edge Cases and Cleanup:**  
  Tests for deregistration, closing accounts, and ensuring proper cleanup.

## 2. **What is Being Tested**

### **A. Account and Token Lifecycle**

- **Market Creation:**  
  Ensures a new market can be created and initialized with the correct admin and parameters.
- **Peach Account Creation:**  
  Verifies that user accounts (Peach accounts) can be created and are owned by the correct wallet.
- **Token Registration:**  
  Checks that new tokens can be registered to a market, with correct mint info, banks, and vaults. Support for Token and Token 2022 Program.
- **Program ID Handling:**  
  Ensures that all token and user objects are initialized with the correct `programId` for legacy/2022 tokens and Kamino instructions.

### **B. Native Lending Flows**

- **Deposits and Withdrawals:**  
  Confirms that users can deposit and withdraw tokens, and that vault balances update as expected.
- **Borrow and Repay:**  
  Ensures users can borrow and repay tokens, and that obligations and vaults reflect these changes.
- **Collateral Fees:**  
  Tests that collateral fees are charged correctly after lending/borrowing actions.

### **C. Kamino Integration**

- **Kamino Lending Flows:**  
  Tests deposit, withdraw, borrow, and repay flows using Kamino reserves and obligations, ensuring correct farm state usage.
- **Obligation Farms:**  
  Ensures that both collateral and debt obligation farm PDAs are derived and initialized for Kamino reserves, and that user objects print both farm addresses for each token.
- **Pre-instructions:**  
  Validates that all Kamino instructions are properly awaited and that pre-instructions (such as reserve refresh) are handled synchronously.

### **D. Account and Token Deregistration**

- **Account Closure:**  
  Verifies that Peach accounts can be closed and that all resources are released.
- **Token Deregistration:**  
  Ensures tokens can be deregistered, and that associated accounts (mint info, banks) are properly cleaned up.
- **Market Closure:**  
  Confirms that a market can be closed after all accounts and tokens are deregistered.

### **E. End-to-End Flows**

- **Full User Journey:**  
  The suite covers a full user journey: market creation, account creation, token registration, deposit, borrow, repay, withdraw, fee charging, and cleanup.
- **Cross-Protocol Actions:**  
  Tests interactions between native Peach and Kamino lending, ensuring state consistency and correct cross-protocol flows.

## 3. **Test Data and Setup**

- **Wallets:**  
  Multiple funded wallets are created for admin and users using the CLI wallet.
- **PDAs:**  
  All relevant PDAs (market, peach account, mint info, banks, vaults, Kamino obligations, farm states, etc.) are derived and validated.
- **Token Accounts:**  
  Associated token accounts are created for program and user wallets, and funded as needed.
- **Oracles:**  
  Oracles are set up or stubbed for price feeds.
- **Farm State:**  
  Both collateral and debt farm states are derived and tested for Kamino tokens.

## 4. **Assertions and Validations**

- **State Checks:**  
  After each action, the test suite fetches on-chain accounts and asserts on their fields (balances, ownership, parameters).
- **Transaction Success:**  
  Each instruction is expected to succeed, and failures are reported with detailed error messages.
- **Cleanup Verification:**  
  After closure/deregistration, the suite checks that accounts are no longer present on-chain.

## 5. **Edge Cases and Error Handling**

- **Duplicate Actions:**  
  Tests ensure that duplicate creation or registration is handled gracefully.
- **Insufficient Funds/Permissions:**  
  Tests for expected failures when users lack funds or permissions.
- **Resource Cleanup:**  
  Ensures that all resources are released and no rent is wasted after tests.
- **Promise Handling:**  
  All async instructions are properly awaited to avoid Promise-wrapped results in test output.

## 6. **Best Practices and Considerations**

- **Always await async instructions** to avoid Promise-wrapped results.
- **Log full object structures** using `console.dir(obj, { depth: null })` for debugging.
- **Test both SPL and 2022 token program flows** by passing the correct `programId`.
- **Test both Kamino collateral and debt farm flows** for each token.
- **Validate all PDAs** (market, peach account, mint info, banks, vaults, farm states) for correctness.
- **Ensure all Kamino and native flows are covered** in both happy and edge-case paths.

## 7. **Test Run Considerations and Kamino Specifics**

### **Single-Run Design**

- The Peach V1 test suite is **crafted to be run once per fresh environment**.  
- If you wish to **re-run the tests with the same program ID and accounts**, you must ensure that all on-chain state (accounts, obligations, balances) is compatible with the test logic.
- **Amounts for deposit, withdraw, borrow, and repay** must be carefully managed and updated if the test is re-run, to avoid failures due to insufficient balances or double initialization.

### **Kamino Pre-Instruction Handling**

- **Kamino pre-instructions** (pre-ix) are dynamically derived based on the current state of reserves and obligations.
- If you change the reserves or the obligation accounts, you **must update the arrays** passed to the pre-instruction logic.
- Refer to the [KLend SDK](https://github.com/kamino-finance/klend-sdk) for details on how Kamino pre-instructions are derived and which accounts are required for each instruction.

### **Collateral/Debt Token Roles**

- The current Kamino test is designed with **USDC as collateral** and **PYUSD as debt**.
- If you wish to **swap these roles** (e.g., use PYUSD as collateral and USDC as debt), you must:
  - Update the test logic to reflect the new roles.
  - **Initialize the correct obligation farms** for both collateral and debt tokens before running Kamino flows.
  - Ensure that all Kamino-related PDAs and farm states are derived and initialized for the new configuration.

### **Running on Devnet vs Mainnet**

- **Kamino tests should be skipped when running on devnet.**  
  Kamino cross-program invocations (CPIs) are only supported on mainnet. Attempting to run Kamino flows on devnet will result in errors or failed transactions.
- Ensure your test runner or scripts detect the cluster and conditionally skip Kamino-related tests if not on mainnet.
- For additional environment-specific considerations, **refer to the `// NOTE:` comments in `peach-v1.ts`**. These comments highlight important differences and caveats when running tests on mainnet versus devnet, such as account funding, CPI support, and initialization requirements.

### **Summary**

- Always check and update on-chain state and test parameters before re-running.
- Adjust pre-instructions and farm initializations as needed for your specific collateral/debt setup.
- Consult the KLend SDK for the latest best practices on Kamino integration and pre-instruction derivation.
- **Skip Kamino tests on devnet; only run them on mainnet for valid results.**
- **Review `// NOTE:` comments in `peach-v1.ts` for further environment-specific guidance.**

## 8. **Summary**

The Peach V1 test suite provides comprehensive coverage of:
- Account and token lifecycle management
- Native and Kamino lending flows (including farm state logic)
- Collateral and fee logic
- Cross-protocol interactions
- Proper cleanup and resource management

This ensures the protocol is robust, safe, and ready for production deployment.
