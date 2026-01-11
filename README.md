# INDEX - in dex we trust

Collection & indexer of core contract deployment addresses of popular DEX protocols across EVM chains

## Getting started

```
cargo build
```

```
cargo run
```

## Goal

I want to have single entrypoint for getting DEX smart contracts addresses for any type of application (smart contract project, aggregator, trading bots, indexers, etc.). For data to be universal I want to store data in universally understood format - JSON

I want to divide data into parts according to following logic:

1. DEX protocol (Uniswap V2, Curve, Balancer V3, PancakeSwap Infinity, etc.)
2. Protocol Deployment on Chain (ethereum, base, arbitrum, bsc, etc.), specified using chain ID to make it universally understood (to avoid confusion of bsc/bnb, ethereum/mainnet and other differences of naming conventions)

For example, Uniswap V3 deployment on base would be stored at following path:

```
protocols/uniswap-v3/8453.json
```

And would contain a map of smart contract identified (human readable name in \_ (what case?) case):

```

```

I want to automatically gather new deployments of major protocols via periodical CI/CD pipeline

## Supported Protocols

To pull list of all DEXes we can utilize repos like [VeloraDEX/paraswap-dex-lib](https://github.com/VeloraDEX/paraswap-dex-lib) or other aggregators integration indexes.

And to filter out "major" ones we can utilize DefiLlama to get two most important metrics for DEX protocols:

-   TVL, $
-   Volume, $

I want to support only a subset of most popular protocols (because there's just too much of them). At the time of writing that would be:

-   Uniswap (V2, V3, V4): get via [briefcase](https://github.com/Uniswap/briefcase) or [contracts](https://github.com/Uniswap/contracts)
-   Curve
-   Balancer (V2, V3): get via [balancer-deployments](https://github.com/balancer/balancer-deployments)
-   SushiSwap (?)
-   PancakeSwap (V2, V3, Infinity): get v3 via [pancake-v3-contracts](https://github.com/pancakeswap/pancake-v3-contracts/tree/5cc479f0c5a98966c74d94700057b8c3ca629afd), v2 via
-   Aerodrome (CPMM, CLMM)
