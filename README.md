# [WAVS](https://docs.wavs.xyz) Earth Search STAC API Oracle

**Oracle for querying and processing Sentinel-2 satellite data**

This project provides a WebAssembly AVS oracle service that queries the [Earth Search STAC API](https://earth-search.aws.element84.com/v1/api.html) for European Space Agency [Sentinel-2](https://en.wikipedia.org/wiki/Sentinel-2) satellite imagery, calculates [NDVI](https://en.wikipedia.org/wiki/Normalized_difference_vegetation_index) (vegetation health index), and stores the results on IPFS. The service is built using Rust and Solidity, configured for Windows _WSL_, Linux, and MacOS.

For detailed documentation on the STAC API Oracle, see [docs/stac-oracle.md](./docs/stac-oracle.md).

**Languages**

- [Rust (this example)](./components/wavs-regen-oracle/)
- [Go](./components/golang-wavs-regen-oracle/README.md)
- [JS / TS](./components/js-wavs-regen-oracle/README.md)

## System Requirements

<details>
<summary>Core (Docker, Compose, Make, JQ, Node v21+)</summary>

### Docker

- **MacOS**: `brew install --cask docker`
- **Linux**: `sudo apt -y install docker.io`
- **Windows WSL**: [docker desktop wsl](https://docs.docker.com/desktop/wsl/#turn-on-docker-desktop-wsl-2) & `sudo chmod 666 /var/run/docker.sock`
- [Docker Documentation](https://docs.docker.com/get-started/get-docker/)

### Docker Compose

- **MacOS**: Already installed with Docker installer
- **Linux + Windows WSL**: `sudo apt-get install docker-compose-v2`
- [Compose Documentation](https://docs.docker.com/compose/)

### Make

- **MacOS**: `brew install make`
- **Linux + Windows WSL**: `sudo apt -y install make`
- [Make Documentation](https://www.gnu.org/software/make/manual/make.html)

### JQ

- **MacOS**: `brew install jq`
- **Linux + Windows WSL**: `sudo apt -y install jq`
- [JQ Documentation](https://jqlang.org/download/)

### Node.js

- **Required Version**: v21+
- [Installation via NVM](https://github.com/nvm-sh/nvm?tab=readme-ov-file#installing-and-updating)
</details>

<details>

<summary>Rust v1.84+</summary>

### Rust Installation

```bash docci-ignore
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

rustup toolchain install stable
rustup target add wasm32-wasip2
```

### Upgrade Rust

```bash docci-ignore
# Remove old targets if present
rustup target remove wasm32-wasi || true
rustup target remove wasm32-wasip1 || true

# Update and add required target
rustup update stable
rustup target add wasm32-wasip2
```

</details>

<details>
<summary>Cargo Components</summary>

### Install Cargo Components

```bash docci-ignore
# Install required cargo components
# https://github.com/bytecodealliance/cargo-component#installation
cargo install cargo-binstall
cargo binstall cargo-component warg-cli wkg --locked --no-confirm --force

# Configure default registry
# Found at: $HOME/.config/wasm-pkg/config.toml
wkg config --default-registry wa.dev
```

</details>

## Create Project

```bash docci-ignore
# if foundry is not installed:
# `curl -L https://foundry.paradigm.xyz | bash && $HOME/.foundry/bin/foundryup`
forge init --template Lay3rLabs/wavs-foundry-template my-wavs --branch main
```

> [!TIP]
> Run `make help` to see all available commands and environment variable overrides.

### Solidity

Install the required packages to build the Solidity contracts. This project supports both [submodules](./.gitmodules) and [npm packages](./package.json).

```bash
# Install packages (npm & submodules)
make setup

# Build the contracts
forge build

# Run the solidity tests
forge test
```

## Build WASI components

Now build the WASI components into the `compiled` output directory.

> [!WARNING]
> If you get: `error: no registry configured for namespace "wavs"`
>
> run, `wkg config --default-registry wa.dev`

> [!WARNING]
> If you get: `failed to find the 'wasm32-wasip1' target and 'rustup' is not available`
>
> `brew uninstall rust` & install it from <https://rustup.rs>

```bash
# This command only builds the rust component.
# Remove `WASI_BUILD_DIR` to build all components.
WASI_BUILD_DIR=components/wavs-regen-oracle make wasi-build
```

## Testing the STAC API Oracle Locally

Test the component locally for business logic validation before on-chain deployment:

```bash
# Build the component
make wasi-build

# Test with default STAC query
make test-stac
```

Expected output:

```shell docci-ignore
Testing STAC API oracle with sample query...
STAC query: {
  "collections": ["sentinel-2-l2a"],
  "bbox": [-122.52, 37.70, -122.35, 37.83],
  "datetime": "2024-06-01T00:00:00Z/2024-06-30T23:59:59Z",
  "limit": 1,
  "query": {
    "eo:cloud_cover": {
      "lt": 10
    }
  }
}
Found 1 features
Processing feature with ID: S2B_10SEG_20240627_0_L2A
Red band URL: https://sentinel-cogs.s3.us-west-2.amazonaws.com/sentinel-s2-l2a-cogs/10/S/EG/2024/6/S2B_10SEG_20240627_0_L2A/B04.tif
NIR band URL: https://sentinel-cogs.s3.us-west-2.amazonaws.com/sentinel-s2-l2a-cogs/10/S/EG/2024/6/S2B_10SEG_20240627_0_L2A/B08.tif
Downloading bands...
Calculating NDVI...
NDVI image uploaded to IPFS: ipfs://Qm...
Metadata uploaded to IPFS: ipfs://Qm...

Result (utf8):
{"metadata_uri":"ipfs://Qm...","feature_id":"S2B_10SEG_20240627_0_L2A"}
```

You can also test with a custom STAC query:

```bash
STAC_QUERY='{"collections":["sentinel-2-l2a"],"bbox":[-122.52,37.70,-122.35,37.83],"datetime":"2024-06-01T00:00:00Z/2024-06-30T23:59:59Z","limit":1,"query":{"eo:cloud_cover":{"lt":10}}}' make test-stac
```

## WAVS

> [!NOTE]
> If you are running on a Mac with an ARM chip, you will need to do the following:
>
> - Set up Rosetta: `softwareupdate --install-rosetta`
> - Enable Rosetta (Docker Desktop: Settings -> General -> enable "Use Rosetta for x86_64/amd64 emulation on Apple Silicon")
>
> Configure one of the following networking:
>
> - Docker Desktop: Settings -> Resources -> Network -> 'Enable Host Networking'
> - `brew install chipmk/tap/docker-mac-net-connect && sudo brew services start chipmk/tap/docker-mac-net-connect`

## Start Environment

Start an ethereum node (anvil), the WAVS service, and deploy [eigenlayer](https://www.eigenlayer.xyz/) contracts to the local network.

```bash docci-background docci-delay-after=15
# Start the backend
#
# This must remain running in your terminal. Use another terminal to run other commands.
# You can stop the services with `ctrl+c`. Some MacOS terminals require pressing it twice.
# make start-all
cp .env.example .env

# Create new operator
cast wallet new-mnemonic --json > .docker/operator1.json
export OPERATOR_MNEMONIC=`cat .docker/operator1.json | jq -r .mnemonic`
export OPERATOR_PK=`cat .docker/operator1.json | jq -r '.accounts[0].private_key'`

make start-all
```

Wait for full local deployment to be ready

```bash docci-delay-after=2
while [ ! -f .docker/start.log ]; do echo "waiting for start.log" && sleep 1; done
```

## Deploy Service Contracts

**Key Concepts:**

- **Trigger Contract:** Any contract that emits events, then WAVS monitors. When a relevant event occurs, WAVS triggers the execution of your WebAssembly component.
- **Submission Contract:** This contract is used by the AVS service operator to submit the results generated by the WAVS component on-chain.

`SERVICE_MANAGER_ADDR` is the address of the Eigenlayer service manager contract. It was deployed in the previous step. Then you deploy the trigger and submission contracts which depends on the service manager. The service manager will verify that a submission is valid (from an authorized operator) before saving it to the blockchain. The trigger contract is any arbitrary contract that emits some event that WAVS will watch for. Yes, this can be on another chain (e.g. an L2) and then the submission contract on the L1 _(Ethereum for now because that is where Eigenlayer is deployed)_.

```bash docci-delay-per-cmd=2
export DEPLOYER_PK=$(cat .nodes/deployer)
export SERVICE_MANAGER_ADDRESS=$(jq -r .addresses.WavsServiceManager .nodes/avs_deploy.json)

forge create SimpleSubmit --json --broadcast -r http://127.0.0.1:8545 --private-key "${DEPLOYER_PK}" --constructor-args "${SERVICE_MANAGER_ADDRESS}" > .docker/submit.json
export SERVICE_SUBMISSION_ADDR=`jq -r .deployedTo .docker/submit.json`

forge create SimpleTrigger --json --broadcast -r http://127.0.0.1:8545 --private-key "${DEPLOYER_PK}" > .docker/trigger.json
export SERVICE_TRIGGER_ADDR=`jq -r .deployedTo .docker/trigger.json`
```

## Deploy Service

Deploy the compiled component with the contract information from the previous steps. Review the [makefile](./Makefile) for more details and configuration options.`TRIGGER_EVENT` is the event that the trigger contract emits and WAVS watches for. By altering `SERVICE_TRIGGER_ADDR` you can watch events for contracts others have deployed.

```bash docci-delay-per-cmd=2
# Build your service JSON
COMPONENT_FILENAME=wavs_regen_oracle.wasm AGGREGATOR_URL=http://127.0.0.1:8001 sh ./script/build_service.sh

# Deploy the service JSON to WAVS so it now watches and submits.
#
# If CREDENTIAL is not set then the default WAVS_CLI .env account will be used
# You can `cast send ${WAVS_SERVICE_MANAGER} 'transferOwnership(address)'` to move it to another account.
SERVICE_CONFIG_FILE=.docker/service.json CREDENTIAL=${DEPLOYER_PK} make deploy-service
```

## Register service specific operator

Each service gets their own key path (hd_path). The first service starts at 1 and increments from there. Get the service ID

```bash
# hack: private key specific to this service
# This is generated from the AVS keys submit mnemonic
# this will be removed in the future. Then we can just --mnemonic-path the different index from source locally
# (where WAVS /service-key returns just the index)
# SERVICE_ID=`curl -s http://localhost:8000/app | jq -r .services[0].id`
# PK=`curl -s http://localhost:8000/service-key/${SERVICE_ID} | jq -rc .secp256k1 | tr -d '[]'`
# AVS_PRIVATE_KEY=`echo ${PK} | tr ',' ' ' | xargs printf "%02x" | tr -d '\n'`

source .env
AVS_PRIVATE_KEY=`cast wallet private-key --mnemonic-path "$WAVS_SUBMISSION_MNEMONIC" --mnemonic-index 1`

# Register the operator with the WAVS service manager
docker run --rm --network host --env-file .env -v ./.nodes:/root/.nodes --entrypoint /wavs/register.sh "ghcr.io/lay3rlabs/wavs-middleware:0.4.0-alpha.5" "$AVS_PRIVATE_KEY"

# Verify registration
docker run --rm --network host --env-file .env -v ./.nodes:/root/.nodes --entrypoint /wavs/list_operator.sh ghcr.io/lay3rlabs/wavs-middleware:0.4.0-alpha.5

# Faucet funds to the aggregator account to post on chain
cast send $(cast wallet address --private-key ${WAVS_AGGREGATOR_CREDENTIAL}) --rpc-url http://localhost:8545 --private-key ${DEPLOYER_PK} --value 1ether
```

## Trigger the STAC API Oracle

Anyone can now call the [trigger contract](./src/contracts/WavsTrigger.sol) which emits the trigger event WAVS is watching for from the previous step. WAVS then calls the service and saves the result on-chain.

```bash
# Trigger with the default STAC query
make trigger-stac

# You can also specify a custom STAC query
STAC_QUERY='{"collections":["sentinel-2-l2a"],"bbox":[-122.52,37.70,-122.35,37.83],"datetime":"2024-06-01T00:00:00Z/2024-06-30T23:59:59Z","limit":1,"query":{"eo:cloud_cover":{"lt":10}}}' make trigger-stac
```

## Show the result

Query the latest submission contract id from the previous request made.

```bash docci-delay-per-cmd=2 docci-output-contains="1"
make get-trigger
```

```bash docci-delay-per-cmd=2
TRIGGER_ID=1 make show-result
```

The result will contain the IPFS URI for the NDVI metadata and the feature ID of the Sentinel-2 image that was processed. You can access the metadata and NDVI visualization through the IPFS URI.
