# Aegis Markets

A Pinocchio-based, Aave V3-inspired Solana devnet lending MVP with Pyth oracle validation and an admin-gated Next.js dashboard.

> Experimental, unaudited software. Do not use with real funds.

See `docs/项目使用说明书.md` for setup, ABI, safety notes and deployment information.

```bash
cargo fmt --all -- --check
cargo test
cargo build-sbf
cd app && pnpm install && pnpm run typecheck && pnpm run build
```

