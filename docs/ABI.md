# Aegis Markets ABI

Program: `4cJDBQmPnuf3GZrP9SW17wDhPpBVcvfNk51MiMrUCCfQ` (Solana devnet)

All integers are unsigned little-endian. Public keys and Pyth feed IDs are 32 raw bytes. This Pinocchio program does not publish or claim an Anchor IDL.

## PDAs

- Config: `findProgramAddress(["config"], programId)`; 72 bytes.
- Reserve: `findProgramAddress(["reserve", mint], programId)`; 224 bytes; also signs for its token vault.
- Position: `findProgramAddress(["position", owner], programId)`; 128 bytes.

## Instructions

| Tag | Instruction | Data after tag | Accounts in order |
|---:|---|---|---|
| 0 | InitializeConfig | none | admin signer/writable; config writable |
| 1 | InitializeReserve | mint, feed ID, decimals u8, LTV u16, liquidation threshold u16, bonus u16, reserve factor u16, supply cap u64, borrow cap u64, max price age u64, max confidence bps u16 | admin signer/writable; config; reserve writable |
| 2 | SetReserve | LTV u16, threshold u16, bonus u16, factor u16, supply cap u64, borrow cap u64, max age u64, max confidence bps u16 (34 bytes after tag) | admin signer; config; reserve writable |
| 3 | InitializePosition | none | user signer/writable; position writable |
| 4 | Deposit | amount u64 | user signer; reserve writable; position writable; user token writable; mint; vault writable; Pyth PriceUpdateV2; SPL Token program |
| 5 | Withdraw | amount u64 | same as Deposit |
| 6 | Borrow | amount u64 | same as Deposit |
| 7 | Repay | amount u64 | same as Deposit |
| 8 | Liquidate | reserved; currently rejected | — |
| 9 | SetPause | paused u8 | admin signer; config; reserve writable |

## Oracle contract

The supplied price account must be owned by the official Pyth Solana Receiver `rec5EKMGg6MxZYaMdyBfgwp4d5rB9T1VQH5pJv5LtFJ`. The program requires a fully verified update, exact configured feed ID, positive price, exponent from -18 through 0, configured maximum age, and a confidence-to-price ratio below the configured basis-point limit.

## Errors

Custom errors 1–16 are respectively: invalid instruction, invalid accounts, missing signature, invalid owner, invalid PDA, already initialized, unauthorized, paused, invalid amount, invalid token account, invalid oracle, stale price, excessive price confidence interval, insufficient liquidity, unhealthy position, arithmetic overflow.

## Important scope note

The current release is a devnet MVP and tracks one aggregate position per wallet. It demonstrates Aave-style supply/borrow controls but is not a byte-for-byte Aave V3 port. Liquidation, cross-reserve health aggregation, interest-index accrual, eMode, isolation debt ceilings, flash loans and Token-2022 transfers require a subsequent audited release.
