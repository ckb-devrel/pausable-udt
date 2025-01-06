use crate::error::Error;
use crate::get_pausable_data;
use crate::utils::{check_owner_mode, collect_inputs_amount, collect_outputs_amount};
use alloc::string::String;
use alloc::vec::Vec;
use ckb_ssri_sdk::public_module_traits::udt::{UDTPausable, UDTPausableData, UDT, UDT_LEN};
use ckb_ssri_sdk::utils::high_level::{
    find_cell_by_out_point, find_cell_data_by_out_point, find_out_point_by_type,
};
use ckb_ssri_sdk::utils::should_fallback;
// use ckb_ssri_sdk_proc_macro::{ssri_method, ssri_module};
use ckb_std::ckb_constants::Source;
use ckb_std::ckb_types::packed::{
    Byte, Byte32, Byte32Vec, BytesVec, BytesVecBuilder, CellDep, CellDepVec, CellDepVecBuilder,
    CellInput, CellInputVec, CellInputVecBuilder, CellOutput, CellOutputBuilder, CellOutputVecBuilder, RawTransactionBuilder, Script, ScriptBuilder,
    ScriptOptBuilder, Transaction, TransactionBuilder, Uint32, Uint64,
};
use ckb_std::ckb_types::{bytes::Bytes, prelude::*};
use ckb_std::debug;
use ckb_std::high_level::{load_cell, load_cell_data, load_cell_type, load_script};
use serde_molecule::{from_slice, to_vec};

pub struct PausableUDT;

// #[ssri_module]
impl UDT for PausableUDT {
    type Error = Error;
    // #[ssri_method(level = "cell")]
    fn balance() -> Result<u128, Error> {
        let data = load_cell_data(0, Source::GroupInput)?;
        let mut buf = [0u8; UDT_LEN];
        if data.len() == UDT_LEN {
            buf.copy_from_slice(&data);
            Ok(u128::from_le_bytes(buf))
        } else {
            Err(Error::Encoding)
        }
    }
    // #[ssri_method(level = "code", transaction = true)]
    fn transfer(
        tx: Option<Transaction>,
        to_lock_vec: Vec<Script>,
        to_amount_vec: Vec<u128>,
    ) -> Result<Transaction, Error> {
        debug!("Entered UDT::transfer");
        if to_amount_vec.len() != to_lock_vec.len() {
            return Err(Error::SSRIMethodsArgsInvalid);
        }
        let tx_builder = match tx {
            Some(ref tx) => tx.clone().as_builder(),
            None => TransactionBuilder::default(),
        };
        let raw_tx_builder = match tx {
            Some(ref tx) => tx.clone().raw().as_builder(),
            None => RawTransactionBuilder::default(),
        };

        let mut cell_output_vec_builder = match tx {
            Some(ref tx) => tx.clone().raw().outputs().as_builder(),
            None => CellOutputVecBuilder::default(),
        };

        for to_lock in to_lock_vec.iter() {
            let new_transfer_output = CellOutputBuilder::default()
                .type_(
                    ScriptOptBuilder::default()
                        .set(Some(load_script()?))
                        .build(),
                )
                .capacity(Uint64::default())
                .lock(to_lock.clone())
                .build();
            cell_output_vec_builder = cell_output_vec_builder.push(new_transfer_output);
        }

        let mut outputs_data_builder = match tx {
            Some(ref tx) => tx.clone().raw().outputs_data().as_builder(),
            None => BytesVecBuilder::default(),
        };

        for to_amount in to_amount_vec.iter() {
            outputs_data_builder = outputs_data_builder.push(to_amount.pack().as_bytes().pack());
        }

        let pausable_data_vec: Vec<UDTPausableData> = Self::enumerate_paused(0, 0)?;
        let mut cell_dep_vec_builder: CellDepVecBuilder = match tx {
            Some(ref tx) => tx.clone().raw().cell_deps().as_builder(),
            None => CellDepVecBuilder::default(),
        };
        for pausable_data in pausable_data_vec {
            let next_type_script_like = pausable_data.next_type_script;
            debug!("Next type script like: {:?}", next_type_script_like);
            if next_type_script_like.is_none() {
                break;
            }
            let next_type_script = ScriptBuilder::default()
                .code_hash(
                    next_type_script_like
                        .clone()
                        .should_be_ok()
                        .code_hash
                        .pack(),
                )
                .hash_type(Byte::new(
                    next_type_script_like.clone().should_be_ok().hash_type,
                ))
                .args(next_type_script_like.clone().should_be_ok().args.pack())
                .build();
            let cell_dep = CellDep::new_builder()
                .out_point(find_out_point_by_type(next_type_script.clone())?)
                .build();
            cell_dep_vec_builder = cell_dep_vec_builder.push(cell_dep);
        }

        Ok(tx_builder
            .raw(
                raw_tx_builder
                    .version(
                        tx.clone()
                            .map(|t| t.raw().version())
                            .unwrap_or_else(|| Uint32::default()),
                    )
                    .cell_deps(cell_dep_vec_builder.build())
                    .header_deps(
                        tx.clone()
                            .map(|t| t.raw().header_deps())
                            .unwrap_or_else(|| Byte32Vec::default()),
                    )
                    .inputs(
                        tx.clone()
                            .map(|t| t.raw().inputs())
                            .unwrap_or_else(|| CellInputVec::default()),
                    )
                    .outputs(cell_output_vec_builder.build())
                    .outputs_data(outputs_data_builder.build())
                    .build(),
            )
            .witnesses(
                tx.clone()
                    .map(|t| t.witnesses())
                    .unwrap_or_else(|| BytesVec::default()),
            )
            .build())
    }

    fn verify_transfer() -> Result<(), Self::Error> {
        debug!("Entered UDT::verify_transfer");
        let inputs_amount = collect_inputs_amount()?;
        let outputs_amount = collect_outputs_amount()?;

        if inputs_amount < outputs_amount {
            return Err(Error::InsufficientBalance);
        }
        debug!("inputs_amount: {}", inputs_amount);
        debug!("outputs_amount: {}", outputs_amount);
        Ok(())
    }

    // #[ssri_method(level = "code")]
    fn name() -> Result<Bytes, Self::Error> {
        Ok(Bytes::from(String::from("PUDT").into_bytes()))
    }

    // #[ssri_method(level = "code")]
    fn symbol() -> Result<Bytes, Self::Error> {
        Ok(Bytes::from(String::from("PUDT").into_bytes()))
    }

    // #[ssri_method(level = "code")]
    fn decimals() -> Result<u8, Self::Error> {
        Ok(6u8)
    }
    // #[ssri_method(level = "script", transaction = true)]
    fn mint(
        tx: Option<Transaction>,
        to_lock_vec: Vec<Script>,
        to_amount_vec: Vec<u128>,
    ) -> Result<Transaction, Error> {
        debug!("Entered UDT::mint");
        let tx_builder = match tx {
            Some(ref tx) => tx.clone().as_builder(),
            None => TransactionBuilder::default(),
        };
        let raw_tx_builder = match tx {
            Some(ref tx) => tx.clone().raw().as_builder(),
            None => RawTransactionBuilder::default(),
        };

        let mut new_cell_output_vec: Vec<CellOutput> = Vec::new();
        let mut new_output_data_vec = Vec::new();
        for (to_lock, to_amount) in to_lock_vec.iter().zip(to_amount_vec.iter()) {
            let new_mint_output = CellOutputBuilder::default()
                .type_(
                    ScriptOptBuilder::default()
                        .set(Some(load_script()?))
                        .build(),
                )
                .lock(to_lock.clone())
                .build();
            new_cell_output_vec.push(new_mint_output);
            new_output_data_vec.push(to_amount);
        }

        let mut cell_output_vec_builder = match tx {
            Some(ref tx) => tx.clone().raw().outputs().as_builder(),
            None => CellOutputVecBuilder::default(),
        };

        for output in new_cell_output_vec {
            cell_output_vec_builder = cell_output_vec_builder.push(output);
        }

        let mut output_data_vec_builder = match tx {
            Some(ref tx) => tx.clone().raw().outputs_data().as_builder(),
            None => BytesVecBuilder::default(),
        };
        for data in new_output_data_vec {
            output_data_vec_builder = output_data_vec_builder.push(data.pack().as_bytes().pack());
        }

        let pausable_data_vec: Vec<UDTPausableData> = Self::enumerate_paused(0, 0)?;
        let mut cell_dep_vec_builder: CellDepVecBuilder = match tx {
            Some(ref tx) => tx.clone().raw().cell_deps().as_builder(),
            None => CellDepVecBuilder::default(),
        };
        for pausable_data in pausable_data_vec {
            let next_type_script_like = pausable_data.next_type_script;
            if next_type_script_like.is_none() {
                break;
            }
            let next_type_script = ScriptBuilder::default()
                .code_hash(
                    next_type_script_like
                        .clone()
                        .should_be_ok()
                        .code_hash
                        .pack(),
                )
                .hash_type(Byte::new(
                    next_type_script_like.clone().should_be_ok().hash_type,
                ))
                .args(next_type_script_like.clone().should_be_ok().args.pack())
                .build();
            let cell_dep = CellDep::new_builder()
                .out_point(find_out_point_by_type(next_type_script.clone())?)
                .build();
            cell_dep_vec_builder = cell_dep_vec_builder.push(cell_dep);
        }

        Ok(tx_builder
            .raw(
                raw_tx_builder
                    .version(
                        tx.clone()
                            .map(|t| t.raw().version())
                            .unwrap_or_else(|| Uint32::default()),
                    )
                    .cell_deps(cell_dep_vec_builder.build())
                    .header_deps(
                        tx.clone()
                            .map(|t| t.raw().header_deps())
                            .unwrap_or_else(|| Byte32Vec::default()),
                    )
                    .inputs(
                        tx.clone()
                            .map(|t| t.raw().inputs())
                            .unwrap_or_else(|| CellInputVec::default()),
                    )
                    .outputs(cell_output_vec_builder.build())
                    .outputs_data(output_data_vec_builder.build())
                    .build(),
            )
            .witnesses(
                tx.clone()
                    .map(|t| t.witnesses())
                    .unwrap_or_else(|| BytesVec::default()),
            )
            .build())
    }

    fn verify_mint() -> Result<(), Self::Error> {
        debug!("Entered UDT::verify_mint");
        let script = load_script()?;
        let args = script.args().unpack();
        if check_owner_mode(&args)? {
            return Ok(());
        } else {
            return Err(Error::NoMintPermission);
        }
    }
    
    fn icon() -> Result<Bytes, Self::Error> {
        // Base64 encoded image
        Ok(Bytes::from(String::from("data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iNDgiIGhlaWdodD0iNDgiIHZpZXdCb3g9IjAgMCA0OCA0OCIgZmlsbD0ibm9uZSIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIj4KPGNpcmNsZSBjeD0iMjQiIGN5PSIyNCIgcj0iMjQiIGZpbGw9IiMwNTFGNDQiLz4KPHBhdGggZD0iTTE1LjM5NzQgMjkuOTMyMUMxNC4xNTk0IDI5LjkzMjEgMTMuMDU5IDI5LjI5NjMgMTIuNDQ4MyAyOC4yMjkzQzExLjgzNzYgMjcuMTYyMyAxMS44NTEzIDI1Ljg5NjEgMTIuNDg2OCAyNC44NDI4TDIzLjA2MTcgNy4yNzQzOEwyMy4wNzU0IDcuMjU1MjhDMjMuNjIyOSA2LjQ1NTcyIDI0LjQ3ODQgNiAyNS40MjIgNkMyNi4yODMxIDYgMjcuMTI3NyA2LjM4NzUgMjcuNjgwNiA3LjAzNjk3QzI4LjIxOTggNy42NzI3OSAyOC40MzcxIDguNDkxNDUgMjguMjg4NiA5LjM0Mjg1TDI2LjA0OTMgMjIuMjI4NUwyMS44Njc3IDI0LjMyN0wyMy43Mjc0IDEzLjUxNTNMMTYuNDI5IDI1Ljk5NDRIMjAuMzY1N0wxOS42MjAyIDI5LjkzMjFIMTUuNDAwMUgxNS4zOTc0WiIgZmlsbD0idXJsKCNwYWludDBfbGluZWFyXzMzMzM5XzkzMTcpIi8+CjxwYXRoIGQ9Ik0yMi40NDg0IDQxLjk5OTVDMjEuNTg3MyA0MS45OTk1IDIwLjc0MjcgNDEuNjEyIDIwLjE4OTggNDAuOTYyNUMxOS42NTA2IDQwLjMyNjcgMTkuNDMzMyAzOS41MDggMTkuNTgxOCAzOC42NTY2TDIxLjgyMTEgMjUuNzcxTDI2LjAwMjcgMjMuNjcyNUwyNC4xNDMgMzQuNDg0MkwzMS40NDE0IDIyLjAwNTFIMjcuNTA0N0wyOC4yNTAyIDE4LjA2NzRIMzIuNDcwM0MzMy43MDgyIDE4LjA2NzQgMzQuODA4NiAxOC43MDMyIDM1LjQxOTQgMTkuNzcwMkMzNi4wMzAxIDIwLjgzNzIgMzYuMDE2MyAyMi4xMDM0IDM1LjM4MDkgMjMuMTU2N0wyNC44MDYgNDAuNzI1MUwyNC43OTIyIDQwLjc0NDJDMjQuMjQ0OCA0MS41NDM4IDIzLjM4OTIgNDEuOTk5NSAyMi40NDU2IDQxLjk5OTVIMjIuNDQ4NFoiIGZpbGw9InVybCgjcGFpbnQxX2xpbmVhcl8zMzMzOV85MzE3KSIvPgo8cGF0aCBvcGFjaXR5PSIwLjE1IiBkPSJNMTkuOTk5OSAyNy45MTY4SDE1LjYyM0MxNC4yNzUgMjcuOTE2OCAxMy40NTI1IDI2LjQ0ODYgMTQuMTYyMyAyNS4zMTM0TDI0LjQzMTggOC44NjM4N0MyNC45MzggOC4wNTA2NyAyNi4yMDYyIDguNTA5MTEgMjYuMDY4NiA5LjQ1MzNMMjQuMDY1OSAyMy4yMTc3TDIxLjg2NzggMjQuMzIyOUwyMy43Mjc1IDEzLjUxMTFMMTYuNDI5MSAyNS45OTAySDIwLjM2NThMMjAuMDAyNyAyNy45MTRMMTkuOTk5OSAyNy45MTY4WiIgZmlsbD0id2hpdGUiLz4KPHBhdGggb3BhY2l0eT0iMC4xNSIgZD0iTTI3Ljg4NDMgMjAuMDYyNUgzMi4yNjExQzMzLjYwOTEgMjAuMDYyNSAzNC40MzE3IDIxLjUzMDYgMzMuNzIxOSAyMi42NjU4TDIzLjQ0OTYgMzkuMTE1NEMyMi45NDM1IDM5LjkyODYgMjEuNjc1MiAzOS40NzAxIDIxLjgxMjggMzguNTI2TDIzLjgxNTUgMjQuNzYxNkwyNi4wMTM2IDIzLjY1NjRMMjQuMTUzOSAzNC40NjgxTDMxLjQ1MjMgMjEuOTg5MUgyNy41MTU2TDI3Ljg3ODggMjAuMDY1MkwyNy44ODQzIDIwLjA2MjVaIiBmaWxsPSJ3aGl0ZSIvPgo8ZGVmcz4KPGxpbmVhckdyYWRpZW50IGlkPSJwYWludDBfbGluZWFyXzMzMzM5XzkzMTciIHgxPSIyMC4xNjQ5IiB5MT0iMjUuNjM0MSIgeDI9IjIwLjE2NDkiIHkyPSIxLjcwMjA1IiBncmFkaWVudFVuaXRzPSJ1c2VyU3BhY2VPblVzZSI+CjxzdG9wIHN0b3AtY29sb3I9IiMwMDU4RjEiLz4KPHN0b3Agb2Zmc2V0PSIwLjUzIiBzdG9wLWNvbG9yPSIjMDBCMUZGIi8+CjxzdG9wIG9mZnNldD0iMSIgc3RvcC1jb2xvcj0iIzAzQTlGRCIvPgo8L2xpbmVhckdyYWRpZW50Pgo8bGluZWFyR3JhZGllbnQgaWQ9InBhaW50MV9saW5lYXJfMzMzMzlfOTMxNyIgeDE9IjI3LjcwMjgiIHkxPSI0MS45OTk1IiB4Mj0iMjcuNzAyOCIgeTI9IjE4LjA2NzQiIGdyYWRpZW50VW5pdHM9InVzZXJTcGFjZU9uVXNlIj4KPHN0b3Agc3RvcC1jb2xvcj0iI0ZGQUIxMSIvPgo8c3RvcCBvZmZzZXQ9IjAuMDkiIHN0b3AtY29sb3I9IiNGREE0MUEiLz4KPHN0b3Agb2Zmc2V0PSIwLjI0IiBzdG9wLWNvbG9yPSIjRkE5MjMyIi8+CjxzdG9wIG9mZnNldD0iMC40MyIgc3RvcC1jb2xvcj0iI0Y0NzQ1QiIvPgo8c3RvcCBvZmZzZXQ9IjAuNjUiIHN0b3AtY29sb3I9IiNFRDRCOTIiLz4KPHN0b3Agb2Zmc2V0PSIwLjkiIHN0b3AtY29sb3I9IiNFMzE3RDkiLz4KPHN0b3Agb2Zmc2V0PSIxIiBzdG9wLWNvbG9yPSIjREYwMEY5Ii8+CjwvbGluZWFyR3JhZGllbnQ+CjwvZGVmcz4KPC9zdmc+Cg==").into_bytes()))
    }
}

// #[ssri_module(base = "UDT")]
impl UDTPausable for PausableUDT {
    // #[ssri_method(level = "cell", transaction = true)]
    fn pause(tx: Option<Transaction>, lock_hashes: &Vec<[u8; 32]>) -> Result<Transaction, Error> {
        let tx_builder = match tx {
            Some(ref tx) => tx.clone().as_builder(),
            None => TransactionBuilder::default(),
        };
        let raw_tx_builder = match tx {
            Some(ref tx) => tx.clone().raw().as_builder(),
            None => RawTransactionBuilder::default(),
        };
        let pausable_data_vec: Vec<UDTPausableData> = Self::enumerate_paused(0, 0)?;

        let new_cell_output: CellOutput;
        let new_output_data: UDTPausableData;
        let mut new_cell_input: Option<CellInput> = None;
        match load_cell(0, Source::GroupInput) {
            Ok(cell_output) => {
                let cell_data = load_cell_data(0, Source::GroupInput)?;
                if cell_data.len() > 0 {
                    debug!("Loaded cell from SSRI Call");
                    new_cell_output = cell_output.clone();
                    let mut pausable_data: UDTPausableData = from_slice(&cell_data, false)?;
                    pausable_data.pause_list.extend(lock_hashes.clone());
                    new_output_data = pausable_data;
                    let out_point =
                        find_out_point_by_type(cell_output.type_().to_opt().should_be_ok())?;
                    new_cell_input =
                        Some(CellInput::new_builder().previous_output(out_point).build());
                } else {
                    debug!("Loaded Dummy Cell from SSRI Call");
                    if pausable_data_vec.len() < 2 {
                        debug!(
                            "Creating the first external pause list cell. Need to attach manually."
                        );
                        new_output_data = UDTPausableData {
                            pause_list: lock_hashes.clone(),
                            next_type_script: None,
                        };
                        new_cell_output = cell_output;
                    } else {
                        debug!("Automatically redirect to the last pause list cell.");
                        let second_last_pausable_data = pausable_data_vec
                            .get(pausable_data_vec.len() - 2)
                            .should_be_ok();
                        let last_cell_type_script = ScriptBuilder::default()
                            .code_hash(
                                second_last_pausable_data
                                    .clone()
                                    .next_type_script
                                    .should_be_ok()
                                    .code_hash
                                    .pack(),
                            )
                            .hash_type(Byte::new(
                                second_last_pausable_data
                                    .clone()
                                    .next_type_script
                                    .should_be_ok()
                                    .hash_type,
                            ))
                            .args(
                                second_last_pausable_data
                                    .clone()
                                    .next_type_script
                                    .should_be_ok()
                                    .args
                                    .pack(),
                            )
                            .build();
                        let last_cell_out_point = find_out_point_by_type(last_cell_type_script)?;
                        new_cell_output = find_cell_by_out_point(last_cell_out_point.clone())?;
                        let last_cell_data =
                            find_cell_data_by_out_point(last_cell_out_point.clone())?;
                        let mut pausable_data: UDTPausableData =
                            from_slice(&last_cell_data, false)?;
                        pausable_data.pause_list.extend(lock_hashes.clone());
                        new_output_data = pausable_data;
                        new_cell_input = Some(
                            CellInput::new_builder()
                                .previous_output(last_cell_out_point)
                                .build(),
                        );
                    }
                }
            }
            Err(err) => {
                return Err(err)?;
            }
        };

        let cell_output_vec_builder = match tx {
            Some(ref tx) => tx.clone().raw().outputs().as_builder(),
            None => CellOutputVecBuilder::default(),
        }
        .push(new_cell_output);

        let output_data_vec_builder = match tx {
            Some(ref tx) => tx.clone().raw().outputs_data().as_builder(),
            None => BytesVecBuilder::default(),
        }
        .push(to_vec(&new_output_data, false)?.pack());

        let mut input_vec_builder = match tx {
            Some(ref tx) => tx.clone().raw().inputs().as_builder(),
            None => CellInputVecBuilder::default(),
        };

        match new_cell_input {
            Some(new_cell_input) => {
                input_vec_builder = input_vec_builder.push(new_cell_input);
            }
            None => {}
        }

        return Ok(tx_builder
            .raw(
                raw_tx_builder
                    .version(
                        tx.clone()
                            .map(|t| t.raw().version())
                            .unwrap_or_else(|| Uint32::default()),
                    )
                    .cell_deps(
                        tx.clone()
                            .map(|t| t.raw().cell_deps())
                            .unwrap_or_else(|| CellDepVec::default()),
                    )
                    .header_deps(
                        tx.clone()
                            .map(|t| t.raw().header_deps())
                            .unwrap_or_else(|| Byte32Vec::default()),
                    )
                    .inputs(input_vec_builder.build())
                    .outputs(cell_output_vec_builder.build())
                    .outputs_data(output_data_vec_builder.build())
                    .build(),
            )
            .witnesses(
                tx.clone()
                    .map(|t| t.witnesses())
                    .unwrap_or_else(|| BytesVec::default()),
            )
            .build());
    }

    // #[ssri_method(level = "cell", transaction = true)]
    fn unpause(tx: Option<Transaction>, lock_hashes: &Vec<[u8; 32]>) -> Result<Transaction, Error> {
        let tx_builder = match tx {
            Some(ref tx) => tx.clone().as_builder(),
            None => TransactionBuilder::default(),
        };
        let raw_tx_builder = match tx {
            Some(ref tx) => tx.clone().raw().as_builder(),
            None => RawTransactionBuilder::default(),
        };
        let pausable_data_vec: Vec<UDTPausableData> = Self::enumerate_paused(0, 0)?;

        let mut new_cell_output: CellOutput;
        let mut new_cell_input: Option<CellInput>;
        let mut output_data_vec_builder = match tx {
            Some(ref tx) => tx.clone().raw().outputs_data().as_builder(),
            None => BytesVecBuilder::default(),
        };
        let mut input_vec_builder = match tx {
            Some(ref tx) => tx.clone().raw().inputs().as_builder(),
            None => CellInputVecBuilder::default(),
        };
        let mut cell_output_vec_builder = match tx {
            Some(ref tx) => tx.clone().raw().outputs().as_builder(),
            None => CellOutputVecBuilder::default(),
        };

        if pausable_data_vec.len() < 2 {
            return Err(Error::SSRIMethodsNotImplemented)?;
        } else {
            for (index, pausable_data) in pausable_data_vec.iter().enumerate() {
                if pausable_data
                    .pause_list
                    .clone()
                    .into_iter()
                    .any(|x| lock_hashes.contains(&x))
                {
                    if index == 0 {
                        return Err(Error::SSRIMethodsNotImplemented)?;
                    }
                    let previous_pausable_data = pausable_data_vec.get(index - 1).should_be_ok();
                    let target_cell_type_script = ScriptBuilder::default()
                        .code_hash(
                            previous_pausable_data
                                .clone()
                                .next_type_script
                                .should_be_ok()
                                .code_hash
                                .pack(),
                        )
                        .hash_type(Byte::new(
                            previous_pausable_data
                                .clone()
                                .next_type_script
                                .should_be_ok()
                                .hash_type,
                        ))
                        .args(
                            previous_pausable_data
                                .clone()
                                .next_type_script
                                .should_be_ok()
                                .args
                                .pack(),
                        )
                        .build();
                    let target_cell_out_point = find_out_point_by_type(target_cell_type_script)?;
                    new_cell_input = Some(
                        CellInput::new_builder()
                            .previous_output(target_cell_out_point.clone())
                            .build(),
                    );
                    input_vec_builder = input_vec_builder.push(new_cell_input.should_be_ok());
                    new_cell_output = find_cell_by_out_point(target_cell_out_point.clone())?;
                    cell_output_vec_builder = cell_output_vec_builder.push(new_cell_output);
                    let mut new_pausable_data = pausable_data.clone();
                    new_pausable_data
                        .pause_list
                        .retain(|x| !lock_hashes.contains(&x));
                    output_data_vec_builder =
                        output_data_vec_builder.push(to_vec(&new_pausable_data, false)?.pack());
                }
            }
        }

        return Ok(tx_builder
            .raw(
                raw_tx_builder
                    .version(
                        tx.clone()
                            .map(|t| t.raw().version())
                            .unwrap_or_else(|| Uint32::default()),
                    )
                    .cell_deps(
                        tx.clone()
                            .map(|t| t.raw().cell_deps())
                            .unwrap_or_else(|| CellDepVec::default()),
                    )
                    .header_deps(
                        tx.clone()
                            .map(|t| t.raw().header_deps())
                            .unwrap_or_else(|| Byte32Vec::default()),
                    )
                    .inputs(input_vec_builder.build())
                    .outputs(cell_output_vec_builder.build())
                    .outputs_data(output_data_vec_builder.build())
                    .build(),
            )
            .witnesses(
                tx.clone()
                    .map(|t| t.witnesses())
                    .unwrap_or_else(|| BytesVec::default()),
            )
            .build());
    }

    // #[ssri_method(level = "code", transaction = false)]
    fn is_paused(lock_hashes: &Vec<[u8; 32]>) -> Result<bool, Error> {
        debug!("Entered is_paused");
        debug!("lock_hashes: {:?}", lock_hashes);

        let mut current_pausable_data = get_pausable_data()?;
        let mut seen_type_hashes: Vec<Byte32> = Vec::new();

        loop {
            // Check current pausable data's pause list
            if current_pausable_data
                .pause_list
                .iter()
                .any(|x| lock_hashes.contains(x))
            {
                return Ok(true);
            }

            // Check for next type script in the chain
            match current_pausable_data.next_type_script {
                Some(next_type_script) => {
                    let next_type_script = Script::new_builder()
                        .code_hash(next_type_script.code_hash.pack())
                        .hash_type(Byte::new(next_type_script.hash_type))
                        .args(next_type_script.args.pack())
                        .build();

                    // Prevent infinite loops
                    if seen_type_hashes.contains(&next_type_script.calc_script_hash()) {
                        return Err(Error::CyclicPauseList);
                    }
                    seen_type_hashes.push(next_type_script.calc_script_hash());

                    // Load next pausable data
                    current_pausable_data = match should_fallback()? {
                        true => {
                            // Fallback logic to find next pausable data in cell deps
                            let mut index = 0;
                            loop {
                                match load_cell_type(index, Source::CellDep) {
                                    Ok(Some(next_pausable_cell_type_script))
                                        if next_pausable_cell_type_script == next_type_script =>
                                    {
                                        break from_slice(
                                            &load_cell_data(index, Source::CellDep)?,
                                            false,
                                        )?
                                    }
                                    Ok(Some(_)) | Ok(None) => index += 1,
                                    Err(_) => return Err(Error::IncompletePauseList),
                                }
                            }
                        }
                        false => {
                            // SSRI path to find next pausable data
                            let next_out_point = find_out_point_by_type(next_type_script)?;
                            from_slice(&find_cell_data_by_out_point(next_out_point)?, false)?
                        }
                    };
                }
                None => break, // No more pausable data cells
            }
        }

        Ok(false)
    }

    // #[ssri_method(level = "code", transaction = false)]
    fn enumerate_paused(mut offset: u64, limit: u64) -> Result<Vec<UDTPausableData>, Error> {
        debug!("Entered enumerate_paused");
        let mut pausable_data_vec: Vec<UDTPausableData> = Vec::new();
        let mut current_pausable_data = get_pausable_data()?;
        let mut seen_type_hashes: Vec<Byte32> = Vec::new();
        let mut entries_counter = 0;

        // Handle initial data
        if current_pausable_data.pause_list.len() < offset as usize {
            offset -= current_pausable_data.pause_list.len() as u64;
        } else {
            let mut modified_data = current_pausable_data.clone();
            modified_data.pause_list = modified_data
                .pause_list
                .into_iter()
                .skip(offset as usize)
                .collect();
            if limit != 0 && modified_data.pause_list.len() as u64 > limit {
                modified_data.pause_list = modified_data
                    .pause_list
                    .into_iter()
                    .take(limit as usize)
                    .collect();
            }
            entries_counter += modified_data.pause_list.len() as u64;
            if entries_counter > 0 {
                pausable_data_vec.push(modified_data);
            }
            offset = 0;
            if limit != 0 && entries_counter >= limit {
                return Ok(pausable_data_vec);
            }
        }
        debug!("Handling chained data and offset: {}", offset);
        while let Some(ref next_type_script) = current_pausable_data.next_type_script {
            let next_type_script: Script = Script::new_builder()
                .code_hash(next_type_script.code_hash.pack())
                .hash_type(Byte::new(next_type_script.hash_type))
                .args(next_type_script.args.pack())
                .build();
            let mut next_pausable_data: Option<UDTPausableData> = None;
            match should_fallback()? {
                true => {
                    let mut index = 0;
                    let mut should_continue = true;
                    while should_continue {
                        match load_cell_type(index, Source::CellDep) {
                            Ok(Some(next_pausable_cell_type_script)) => {
                                debug!(
                                    "Loaded cell type script: {:?}",
                                    next_pausable_cell_type_script
                                );
                                if next_pausable_cell_type_script == next_type_script {
                                    should_continue = false;
                                    next_pausable_data = Some(from_slice(
                                        &load_cell_data(index, Source::CellDep)?,
                                        false,
                                    )?);
                                } else {
                                    index += 1;
                                    debug!("Incrementing index: {}", index);
                                }
                            }
                            Ok(None) => {
                                index += 1;
                            }
                            Err(_) => {
                                should_continue = false;
                            }
                        }
                    }
                }
                false => {
                    debug!(
                        "Loading external pause list cell from SSRI Call: {:?}",
                        next_type_script
                    );
                    let next_out_point = find_out_point_by_type(next_type_script.clone())?;
                    debug!("Found out point: {:?}", next_out_point);
                    next_pausable_data = Some(from_slice(
                        &find_cell_data_by_out_point(next_out_point)?,
                        false,
                    )?);
                    debug!("Loaded next pausable data: {:?}", next_pausable_data);
                }
            };
            match next_pausable_data {
                Some(mut next_pausable_data) => {
                    debug!("Loaded next pausable data: {:?}", next_pausable_data);
                    if seen_type_hashes
                        .clone()
                        .into_iter()
                        .any(|x| x == next_type_script.calc_script_hash())
                    {
                        return Err(Error::CyclicPauseList)?;
                    } else {
                        debug!(
                            "Adding next type script to seen list: {:?}",
                            next_type_script
                        );
                        seen_type_hashes.push(next_type_script.calc_script_hash());
                    }

                    if next_pausable_data.pause_list.len() < offset as usize {
                        offset -= next_pausable_data.pause_list.len() as u64;
                        current_pausable_data = next_pausable_data;
                    } else {
                        next_pausable_data.pause_list = next_pausable_data
                            .pause_list
                            .into_iter()
                            .skip(offset as usize)
                            .collect();
                        pausable_data_vec.push(next_pausable_data.clone());
                        entries_counter += next_pausable_data.pause_list.len() as u64;
                        offset = 0;
                        if limit != 0 && entries_counter >= limit {
                            break;
                        }
                        current_pausable_data = next_pausable_data;
                    }
                }
                None => {
                    return Err(Error::IncompletePauseList)?;
                }
            }
        }

        debug!("Handling limit: {}", limit);
        if entries_counter > limit && limit != 0 {
            if let Some(last) = pausable_data_vec.last_mut() {
                last.pause_list = last
                    .pause_list
                    .clone()
                    .into_iter()
                    .take(last.pause_list.len() - (entries_counter - limit) as usize)
                    .collect();
            }
        }
        Ok(pausable_data_vec)
    }
}
