# farmore-ethereum-adapter

> **Status: mainnet is not live yet.** Farmore is in pre-mainnet development (testnet at
> Stage 2).

The **reference EVM adapter** for [Farmore](https://farmore.network), the implementation
of the [`farmore-core`](https://github.com/farmore-network/farmore-core) `ChainAdapter`
trait for Ethereum, Base, and other EVM chains. It also ships the typed home-chain
contract bindings (`ISettlement`, `INamespace`, `IBondVault`, ERC-20) used by the node and
resolver. Built on [`alloy`](https://github.com/alloy-rs/alloy).

This crate is the **template** a third-party `farmore-<chain>-adapter` copies: it shows how
to front recipients from the operator's own account (`Settler`), verify fills to finality
(`Verifier`), and declare capabilities (`Capable`), all behind the chain-neutral trait.

## Use it

After this crate is published, depend on the registry version (preferred), or pin the git
tag:

```toml
# released:
farmore-ethereum-adapter = "0.1"
# or pinned to git (reproducible before a crates.io release):
farmore-ethereum-adapter = { git = "https://github.com/farmore-network/farmore-ethereum-adapter", tag = "v0.1.0" }
```

## Build & test

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

### Cross-repo dependency model

This crate depends on `farmore-core` via a **path + version** dependency:
`farmore-core = { path = "../farmore-core", version = "0.1.0" }`. Check out the sibling
repos adjacently and everything builds with no edits:

```
farmore/
  farmore-core/
  farmore-ethereum-adapter/   <- you are here
```

`cargo publish` drops the path and resolves `farmore-core` from the registry, so the
committed manifest is release-ready. Consumers who don't keep siblings simply depend on the
published/tagged version shown above.

## License

MIT — see [LICENSE](LICENSE).
