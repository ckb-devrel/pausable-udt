# `pausable-udt`

> [`[EN/CN] Script-Sourced Rich Information - 来源于 Script 的富信息`](https://talk.nervos.org/t/en-cn-script-sourced-rich-information-script/8256): General introduction to SSRI.
>
> [`pausable-udt`](https://github.com/Alive24/pausable-udt): The first fully SSRI compliant and production ready contract that exemplifies all use cases that SSRI protocool covers.
>
> [`pausable-udt` - Audit Report](https://github.com/Alive24/pausable-udt/blob/master/20241224-Pausable-UDT-Final-Audit-Report.pdf)
>
> [`ssri-server`](https://github.com/ckb-devrel/ssri-server): Server for calling SSRI methods.
>
> [`ckb_ssri_sdk`](https://github.com/Alive24/ckb_ssri_sdk) : Toolkit to help developers build SSRI-Compliant smart contracts on CKB by providing public Module Traits which would receive first party infrastructure support across the ecosystem, such as CKB Explorer, JoyID wallet, etc, and useful utility functions and macros to simplify the experience of building SSRI-Compliant contract.
>
> [`ckb_ssri_cli`](https://github.com/Alive24/ckb_ssri_cli): Command Line Interface for general users, moderators, and devs to interact with SSRI-Compliant Contracts deployed on CKB Network. Also exemplifies how to interact with SSRI compliant contract in Node.js.
>
> [`ssri-test`](https://github.com/Hanssen0/ssri-test): First prototype of SSRI-Compliant contract.

This is a SSRI-compliant smart contract that implements a pausable UDT (User-Defined Token) with the SSRI protocol. By maintaining and referring to the `UDTPausableData` , transactions involving UDT would in effect pause minting, transferring, and burning behaviors when any paused lock hash is involved in the transaction. Moreover, the pause list would be publicly accessible and readable to the general public for any decentralized financial arrangements.

## Background

As xUDT is in effect deprecated in terms of providing extensibility for UDT, the need to extend UDT contracts are still to be satisfied; while the programmability of CKB allows great diversities in the way to implement, the inevitable need to index and interact in activities involving UDT assets requires a unified protocol to provide discoverability and predictability for both generic users and developers to explore new possibilities on the basis of trust on the behaviors of the infrastructures and other actors within the CKB and the greater connect ecology.

Pausable UDT is one of the urgent demands of the market that would be good examples of the situation: We would want the smart contracts to validate transactions based on the pause list provided by the project; at the same time, we would need a way for generic users to access the pause list to build trust on the project to invest and trade within.

Based on the experience and insights, as well as the latest updates of utilities and framework including `SSRI`, we would want to design Pausable UDT in a way that is public, intuitive, predicable, and indeed extensible, and develop it in a way that both promises security and functionalities of `Pausable UDT` in the first place and exemplify a more reliable and intuitive way of building smart contracts on CKB-VM.

## Quick Note on SSRI

SSRI stands for `Script Sourced Rich Information`; it is a protocol for strong bindings of relevant information and conventions to the Script itself on CKB. For more information, please read [[EN/CN] Script-Sourced Rich Information - 来源于 Script 的富信息](https://talk.nervos.org/t/en-cn-script-sourced-rich-information-script/8256).

Such bindings would take place in a progressive pattern:

1. On the level of validating transactions, by specifically using Rust Traits, we recognize the purpose (or more specifically, the `Intent` of running the script) (e.g., `minting UDT`, `transferring`) and build relevant validation logics within the scope of the corresponding method.
2. On the level of reading and organizing contract code, by selectively implementing methods of public module traits (e.g. `UDT`, `UDTPausable`) in combinations, generic users and devs would be able to quickly understand and organize functionalities of contracts as well as the relevant adaptations / integrations in dApps , especially in use cases involving multiple distinct contracts (and very likely from different projects) within same transactions.
3. On the level of dApp integration and interactions with `ckb_ssri_cli`, SSRI-Compliant contracts provide predictable interfaces for information query (e.g. generic metadata source for explorer, CCC integration for pubic trait methods such as UDT), transaction generation/completion, and output data calculations which reduces engineering workload significantly by sharing code effectively.

## Interfaces

We will be implementing public module traits `UDT` defined in `ckb_ssri_sdk` . This would be the basis of code organizing, public presenting, and generic integrations to dApps at the moment, and method reflections for SSRI-Calling in the future.

- For methods that we do not plan to implement, we will just simply return `SSRIError::SSRIMethodsNotImplemented` .
- Methods that corresponds to a behavior (e.g. mint, transfer) would return an incomplete `Transaction` while you need to fill in the missing inputs and `CellDeps` with CCC. It can also be provided in the parameters in a way that allows chaining multiple actions.

```rust
pub trait UDT {
    type Error;
    fn balance() -> Result<u128, Self::Error>;
    fn transfer(
        tx: Option<Transaction>,
        to_lock_vec: Vec<Script>,
        to_amount_vec: Vec<u128>,
    ) -> Result<Transaction, Self::Error>;
    fn verify_transfer() -> Result<(), Self::Error>;
    fn name() -> Result<Bytes, Self::Error>;
    fn symbol() -> Result<Bytes, Self::Error>;
    fn decimals() -> Result<u8, Self::Error>;
    fn mint(
        tx: Option<Transaction>,
        to_lock_vec: Vec<Script>,
        to_amount_vec: Vec<u128>,
    ) -> Result<Transaction, Self::Error>;
    fn verify_mint() -> Result<(), Self::Error>;
}

pub enum UDTError {
    InsufficientBalance,
    NoMintPermission,
    NoBurnPermission,
}

pub trait UDTPausable: UDT {
    fn pause(
        tx: Option<Transaction>,
        lock_hashes: &Vec<[u8; 32]>,
    ) -> Result<Transaction, Self::Error>;
    fn unpause(
        tx: Option<Transaction>,
        lock_hashes: &Vec<[u8; 32]>,
    ) -> Result<Transaction, Self::Error>;
    fn is_paused(lock_hashes: &Vec<[u8; 32]>) -> Result<bool, Self::Error>;
    fn enumerate_paused(offset: u64, limit: u64) -> Result<Vec<UDTPausableData>, Self::Error>;
}

pub enum UDTPausableError {
    NoPausePermission,
    NoUnpausePermission,
    AbortedFromPause,
    IncompletePauseList,
    CyclicPauseList,
}
```

## Script `<pausable-udt>`

- This project would only introduce one new `Script` as the asset type script.
- To be compatible with those UDT issuance that would take place before Script `<pausable-udt>` and scheduled to upgrade when it becomes available, we would use the same rule for args definition as what sUDT/xUDT requires: if at least one input cell in the transaction uses owner lock specified by the `pausable-udt`as its cell lock, it enters governance operation and minting would be allowed.
- By default, the contract code itself maintains a pause list of lock hashes that can only be updated by upgrading; if necessary, we can also maintain external lists of lock hashes in extra cell with Type ID implementation and point to them at `UDTPausableData.next_type_script` in a chained pattern.

## Data Structures

```rust
use serde::{Serialize, Deserialize};
use serde_molecule::{to_vec, from_slice};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UDTPausableData {
    pub pause_list: Vec<[u8; 32]>,
    pub next_type_script: Option<ScriptLike>
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ScriptLike {
    pub code_hash: [u8; 32],
    pub hash_type: u8,
    pub args: Vec<u8>,
}
```

## User and Admin Experience

### Recipes

### Transfer

```yaml
Inputs:
    pausable-udt-cell:
        Type:
            code: <pausable-udt>
            args: <owner lock script hash>
        Lock: <User Lock A (Not Paused)>
        Data: <amount>
Dependencies:
    external-pausableData-cell: # Only if external pause list is enabled.
        Data: UDTPausableData
Outputs:
  pausable-udt-cell:
        Type:
            code: <pausable-udt>
            args: <owner lock script hash>
        Lock: <User Lock B (Not Paused)>
        Data: <transferred-amount>
    pausable-udt-cell:
        Type:
            code: <pausable-udt>
            args: <owner lock script hash>
        Lock: <User Lock A (Not Paused)>
        Data: <change-amount>
```

- While transferring, transactions must make sure that none of the user lock script hashes in the transaction is included (“paused”); otherwise, the transaction would return `UDTPausableError::AbortedFromPause` . In case of using external pausable data cell, all transactions must include all the external pausable data cell in the `CellDep` .

### Pause / Unpause (Only Available if using external pausable data cell)

```yaml
Inputs:
    proxy-lock-cell:
      Type:
            code: <Type ID Type>
            args: <Type ID>
        Lock:     <Multisig>
    external-pausable-data-cell:
        Type:
            code: <Type ID Type>
            args: <Type ID>
        Lock:
          code: <Proxy Lock>
          args: <Proxy Lock Cell type hash>
        Data: UDTMetadataData
Dependencies:
Outputs:
  proxy-lock-cell:
      Type:
            code: <Type ID Type>
            args: <Type ID>
        Lock:     <Multisig>
    external-pausable-data-cell:
        Type:
            code: <Type ID Type>
            args: <Type ID>
        Lock:
          code: <Proxy Lock>
          args: <Proxy Lock Type ID>
        Data: UDTMetadataData
```

- By adding / removing lock hashes to the pause list, admins can modify the pausing policies according to the specifications of the project.

## Interacting with `ckb-ssri-cli` (or anything with TypeScript)

- See examples in <https://github.com/Alive24/ckb_ssri_cli>. It would be transferrable to any TypeScript project.
- You would need to run an <https://github.com/Alive24/ssri-server> locally at the moment.

```tsx
// Mint
const payload = {
  id: 2,
  jsonrpc: "2.0",
  method: "run_script_level_script",
  params: [
    codeCellDep.outPoint.txHash,
    Number(codeCellDep.outPoint.index),
    [
      mintPathHex,
      `0x${heldTxEncodedHex}`,
      `0x${toLockArrayEncodedHex}`,
      `0x${toAmountArrayEncodedHex}`,
    ],
    // NOTE: field names are wrong when using udtTypeScript.toBytes()
    {
      code_hash: udtTypeScript.codeHash,
      hash_type: udtTypeScript.hashType,
      args: udtTypeScript.args,
    },
  ],
};

// Transfer
const payload = {
  id: 2,
  jsonrpc: "2.0",
  method: "run_script_level_script",
  params: [
    codeCellDep.outPoint.txHash,
    Number(codeCellDep.outPoint.index),
    // args.index,
    [
      transferPathHex,
      `0x${heldTxEncodedHex}`,
      `0x${toLockArrayEncodedHex}`,
      `0x${toAmountArrayEncodedHex}`,
    ],
    // NOTE: field names are wrong when using udtTypeScript.toBytes()
    {
      code_hash: udtTypeScript.codeHash,
      hash_type: udtTypeScript.hashType,
      args: udtTypeScript.args,
    },
  ],
};

// Pause (Specific Node on the external pausable data cell chain
const payload = {
  id: 2,
  jsonrpc: "2.0",
  method: "run_script_level_cell",
  params: [
    codeCellDep.outPoint.txHash,
    Number(codeCellDep.outPoint.index),
    [pausePathHex, `0x${heldTxEncodedHex}`, `0x${lockHashU832ArrayEncodedHex}`],
    {
      cell_output: {
        capacity: ccc.numToHex(0),
        lock: {
          code_hash: targetPausableDataCell.cellOutput.lock.codeHash,
          args: targetPausableDataCell.cellOutput.lock.args,
          hash_type: targetPausableDataCell.cellOutput.lock.hashType,
        },
        type: {
          code_hash: targetPausableDataCell.cellOutput.type?.codeHash,
          args: targetPausableDataCell.cellOutput.type?.args,
          hash_type: targetPausableDataCell.cellOutput.type?.hashType,
        },
      },
      hex_data: targetPausableDataCell.outputData,
    },
  ],
};

// Pause (Autoredirecting to latest external pausable data cell)
let dummy_typeid_script = await ccc.Script.fromKnownScript(
  client,
  ccc.KnownScript.TypeId,
  "0x"
);
const payload = {
  id: 2,
  jsonrpc: "2.0",
  method: "run_script_level_cell",
  params: [
    codeCellDep.outPoint.txHash,
    Number(codeCellDep.outPoint.index),
    [pausePathHex, `0x${heldTxEncodedHex}`, `0x${lockHashU832ArrayEncodedHex}`],
    {
      cell_output: {
        capacity: `0x0`,
        lock: {
          code_hash: ownerLock.codeHash,
          args: ownerLock.args,
          hash_type: ownerLock.hashType,
        },
        type: {
          code_hash: dummy_typeid_script.codeHash,
          args: dummy_typeid_script.args,
          hash_type: dummy_typeid_script.hashType,
        },
      },
      hex_data: `0x`,
    },
  ],
};

// Unpause
const payload = {
  id: 2,
  jsonrpc: "2.0",
  method: "run_script_level_code",
  params: [
    codeCellDep.outPoint.txHash,
    Number(codeCellDep.outPoint.index),
    [
      unpausePathHex,
      `0x${heldTxEncodedHex}`,
      `0x${lockHashU832ArrayEncodedHex}`,
    ],
  ],
};

// Enumerate Paused
const payload = {
  id: 2,
  jsonrpc: "2.0",
  method: "run_script_level_code",
  params: [
    codeCellDep.outPoint.txHash,
    Number(codeCellDep.outPoint.index),
    [enumeratePausedPathHex, `0x${offsetHex}`, `0x${limitHex}`],
  ],
};

// Is Paused?
const payload = {
  id: 2,
  jsonrpc: "2.0",
  method: "run_script_level_code",
  params: [
    codeCellDep.outPoint.txHash,
    Number(codeCellDep.outPoint.index),
    // args.index,
    [isPausedPathHex, `0x${lockHashU832ArrayEncodedHex}`],
  ],
};

// Decimals
const payload = {
  id: 2,
  jsonrpc: "2.0",
  method: "run_script_level_code",
  params: [
    matchingCellDep.outPoint.txHash,
    Number(matchingCellDep.outPoint.index),
    [decimalPathHex],
  ],
};

// Name
const payload = {
  id: 2,
  jsonrpc: "2.0",
  method: "run_script_level_code",
  params: [
    matchingCellDep.outPoint.txHash,
    Number(matchingCellDep.outPoint.index),
    [namePathHex],
  ],
};

// Symbol
const payload = {
  id: 2,
  jsonrpc: "2.0",
  method: "run_script_level_code",
  params: [
    matchingCellDep.outPoint.txHash,
    Number(matchingCellDep.outPoint.index),
    [symbolPathHex],
  ],
};

// Get transaction and send it
// Send POST request
try {
  const response = await axios.post(process.env.SSRI_SERVER_URL!, payload, {
    headers: { "Content-Type": "application/json" },
  });
  const mintTx = blockchain.Transaction.unpack(response.data.result);
  const cccMintTx = ccc.Transaction.from(mintTx);
  await cccMintTx.completeInputsByCapacity(signer);
  await cccMintTx.completeFeeBy(signer);
  const mintTxHash = await signer.sendTransaction(cccMintTx);
  this.log(
    `Mint ${args.toAmount} ${args.symbol} to ${args.toAddress}. Tx hash: ${mintTxHash}`
  );
} catch (error) {
  console.error("Request failed", error);
}
```

## Testing

- Due to the limitations of `ckb_testtools`, it is recommended to test the same SSRI-Compliant Contract on two level:
  - On-chain Verification: Test with `ckb_testtools`
  - Off-chain Query/Integration, Transaction Generations/Completions: Test with `ckb_ssri_cli` against the latest deployment.

## Deployment and Migration

- Deploy and upgrade with [ckb-cinnabar](https://github.com/ashuralyk/ckb-cinnabar?tab=readme-ov-file#deployment-module) for easier deployment and migration with Type ID.

```bash
ckb-cinnabar deploy --contract-name pausable-udt --tag transaction.v241112 --payer-address ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsqtxe0gs9yvwrsc40znvdc6sg4fehd2mttsngg4t4 --type-id

ckb-cinnabar migrate --contract-name pausable-udt --from-tag v241030.1 --to-tag v241030.2
```

## Roadmaps and Goal

- [x] Equivalent functionalities of sUDT in pure Rust;
- [x] Validations of UDT transactions in fallback function on predefined paused locks hashes;
- [x] First integration with dApps for the purpose of demonstration with CCC.
- [x] Fully supported SSRI protocol
