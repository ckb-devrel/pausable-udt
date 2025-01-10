#![no_std]
#![cfg_attr(not(test), no_main)]

#[cfg(test)]
extern crate alloc;

use alloc::borrow::Cow;
use alloc::ffi::CString;
use ckb_ssri_sdk::prelude::decode_u8_32_vector;

use ckb_ssri_sdk::utils::should_fallback;
use ckb_ssri_sdk_proc_macro::ssri_methods;
use ckb_std::ckb_types::packed::{Byte32, Bytes, Script, ScriptBuilder, Transaction};
use ckb_std::debug;
#[cfg(not(test))]
use ckb_std::default_alloc;
#[cfg(not(test))]
ckb_std::entry!(program_entry);
#[cfg(not(test))]
default_alloc!();

use ckb_ssri_sdk::public_module_traits::udt::{ScriptLike, UDTPausable, UDTPausableData, UDT};

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use ckb_std::ckb_types::prelude::{Builder, Entity, Pack};
use ckb_std::high_level::decode_hex;
use ckb_std::syscalls::{pipe, write};
use config::*;
use error::Error;

mod config;
mod error;
mod fallback;
mod modules;
mod molecule;
mod utils;
use ::molecule::prelude::Reader;

pub fn get_pausable_data() -> Result<UDTPausableData, Error> {
    debug!("Entered get_pausable_data");
    Ok(UDTPausableData {
        pause_list: utils::format_pause_list(
            IN_CONTRACT_PAUSED_LOCK_HASHES.iter().copied().collect(),
        ),
        next_type_script: if INITIAL_EXTERNAL_DATA_CELL_TYPE_CODE_HASH.is_empty()
            || INITIAL_EXTERNAL_DATA_CELL_TYPE_ARGS.is_empty()
        {
            None
        } else {
            Some(ScriptLike {
                code_hash: decode_hex(
                    &CString::new(INITIAL_EXTERNAL_DATA_CELL_TYPE_CODE_HASH)
                        .map_err(|_| Error::InvalidPauseData)?
                        .as_c_str()[2..],
                )?
                .try_into()
                .map_err(|_| Error::InvalidPauseData)?,
                args: decode_hex(
                    &CString::new(INITIAL_EXTERNAL_DATA_CELL_TYPE_ARGS)
                        .map_err(|_| Error::InvalidPauseData)?
                        .as_c_str()[2..],
                )?,
                hash_type: 1u8.into(),
            })
        },
    })
}

fn program_entry_wrap() -> Result<(), Error> {
    let argv = ckb_std::env::argv();

    if should_fallback()? {
        return Ok(fallback::fallback()?);
    }

    debug!("Entering ssri_methods");
    // NOTE: The following part is an entry function acting as an controller for all SSRI methods and also handles the deserialization/serialization.
    // In the future, methods can be reflected automatically from traits using procedural macros and entry methods to other methods of the same trait for a more concise and maintainable entry function.
    let res: Cow<'static, [u8]> = ssri_methods!(
        argv: &argv,
        invalid_method: Error::SSRIMethodsNotFound,
        invalid_args: Error::SSRIMethodsArgsInvalid,
        "UDT.name" => Ok(Cow::from(modules::PausableUDT::name()?.to_vec())),
        "UDT.symbol" => Ok(Cow::from(modules::PausableUDT::symbol()?.to_vec())),
        "UDT.decimals" => Ok(Cow::from(modules::PausableUDT::decimals()?.to_le_bytes().to_vec())),
        "UDT.icon" => Ok(Cow::from(modules::PausableUDT::icon()?.to_vec())),
        "UDTPausable.is_paused" => {
            let response = modules::PausableUDT::is_paused(&decode_u8_32_vector(decode_hex(argv[1].as_ref())?).map_err(|_|error::Error::SSRIMethodsArgsInvalid)?)?;
            let response_bytes = response.iter().map(|b| if *b { 1 } else { 0 }).collect::<Vec<u8>>().pack();
            Ok(Cow::from(response_bytes.as_bytes().to_vec()))
        },
        "UDTPausable.enumerate_paused" => {
            let offset = u64::from_le_bytes(decode_hex(argv[1].as_ref())?.try_into().unwrap_or_default());
            let limit = u64::from_le_bytes(decode_hex(argv[2].as_ref())?.try_into().unwrap_or_default());
            let response = modules::PausableUDT::enumerate_paused(offset, limit)?;
            Ok(Cow::from(response.as_bytes().to_vec()))
        },
        "UDT.transfer" => {
            debug!("program_entry_wrap | Entered UDT.transfer");
            let to_lock_vec_molecule = molecule::ScriptVec::from_slice(decode_hex(argv[2].as_ref())?.as_slice()).map_err(|_|Error::MoleculeVerificationError)?;
            let mut to_lock_vec: Vec<Script> = vec![];
            for script in to_lock_vec_molecule.into_iter() {
                let parsed_script = ScriptBuilder::default()
                    .code_hash(Byte32::from_slice(script.as_reader().code_hash().to_entity().as_slice()).map_err(|_|Error::MoleculeVerificationError)?)
                    .hash_type(script.as_reader().hash_type().to_entity())
                    .args(Bytes::from_slice(script.as_reader().args().to_entity().as_slice()).map_err(|_|Error::MoleculeVerificationError)?)
                    .build();
                to_lock_vec.push(parsed_script);
            }

            let to_amount_bytes = decode_hex(argv[3].as_ref())?;
            let to_amount_vec: Vec<u128> = to_amount_bytes[4..]
                .chunks(16)
                .map(|chunk| {
                    return u128::from_le_bytes(chunk.try_into().unwrap())}
                )
                .collect();

            if argv[2].is_empty() || argv[3].is_empty() || to_lock_vec.len() != to_amount_vec.len() {
                Err(Error::SSRIMethodsArgsInvalid)?;
            }

            let tx: Option<Transaction>;
            if argv[1].as_ref().to_str()? == "" {
                tx = None;
            } else {
                let parsed_tx: Transaction = Transaction::from_compatible_slice(&decode_hex(argv[1].as_ref())?).map_err(|_|Error::MoleculeVerificationError)?;
                tx = Some(parsed_tx);
            }

            Ok(Cow::from(modules::PausableUDT::transfer(tx, to_lock_vec, to_amount_vec)?.as_bytes().to_vec()))
        },
        "UDT.mint" => {
            debug!("program_entry_wrap | Entered UDT.mint");
            let to_lock_vec_molecule = molecule::ScriptVec::from_slice(decode_hex(argv[2].as_ref())?.as_slice()).map_err(|_|Error::MoleculeVerificationError)?;
            let mut to_lock_vec: Vec<Script> = vec![];
            for script in to_lock_vec_molecule.into_iter() {
                let parsed_script = ScriptBuilder::default()
                    .code_hash(Byte32::from_slice(script.as_reader().code_hash().to_entity().as_slice()).map_err(|_|Error::MoleculeVerificationError)?)
                    .hash_type(script.as_reader().hash_type().to_entity())
                    .args(Bytes::from_slice(script.as_reader().args().to_entity().as_slice()).map_err(|_|Error::MoleculeVerificationError)?)
                    .build();
                to_lock_vec.push(parsed_script);
            }
            debug!("program_entry_wrap | to_lock_vec: {:?}", to_lock_vec);

            let to_amount_bytes = decode_hex(argv[3].as_ref())?;
            let to_amount_vec: Vec<u128> = to_amount_bytes[4..]
                .chunks(16)
                .map(|chunk| {
                    return u128::from_le_bytes(chunk.try_into().unwrap())}
                )
                .collect();
            debug!("program_entry_wrap | to_amount_vec: {:?}", to_amount_vec);

            if argv[2].is_empty() || argv[3].is_empty() || to_lock_vec.len() != to_amount_vec.len() {
                Err(Error::SSRIMethodsArgsInvalid)?;
            }

            let tx: Option<Transaction>;
            if argv[1].as_ref().to_str()? == ""{
                tx = None;
            } else {
                let parsed_tx: Transaction = Transaction::from_compatible_slice(&decode_hex(argv[1].as_ref())?).map_err(|_|Error::MoleculeVerificationError)?;
                tx = Some(parsed_tx);
            }

            Ok(Cow::from(modules::PausableUDT::mint(tx, to_lock_vec, to_amount_vec)?.as_bytes().to_vec()))
        },
        "UDTPausable.pause" => {
            debug!("program_entry_wrap | Entered UDTPausable.pause");
            let lock_hashes_vec: Vec<[u8; 32]> = decode_u8_32_vector(decode_hex(argv[2].as_ref())?).map_err(|_|error::Error::InvalidArray)?;
            debug!("program_entry_wrap | lock_hashes_vec: {:?}", lock_hashes_vec);

            if argv[2].is_empty() {
                Err(Error::SSRIMethodsArgsInvalid)?;
            }

            let tx: Option<Transaction>;
            if argv[1].as_ref().to_str()? == ""{
                tx = None;
            } else {
                let parsed_tx: Transaction = Transaction::from_compatible_slice(&decode_hex(argv[1].as_ref())?).map_err(|_|Error::MoleculeVerificationError)?;
                tx = Some(parsed_tx);
            }

            Ok(Cow::from(modules::PausableUDT::pause(tx, &lock_hashes_vec)?.as_bytes().to_vec()))
        },
        "UDTPausable.unpause" => {
            debug!("program_entry_wrap | Entered UDTPausable.unpause");
            let lock_hashes_vec: Vec<[u8; 32]> = decode_u8_32_vector(decode_hex(argv[2].as_ref())?).map_err(|_|error::Error::InvalidArray)?;
            debug!("program_entry_wrap | lock_hashes_vec: {:?}", lock_hashes_vec);

            if argv[2].is_empty() {
                Err(Error::SSRIMethodsArgsInvalid)?;
            }

            let tx: Option<Transaction>;
            if argv[1].as_ref().to_str()? == ""{
                tx = None;
            } else {
                let parsed_tx: Transaction = Transaction::from_compatible_slice(&decode_hex(argv[1].as_ref())?).map_err(|_|Error::MoleculeVerificationError)?;
                tx = Some(parsed_tx);
            }
            Ok(Cow::from(modules::PausableUDT::unpause(tx, &lock_hashes_vec)?.as_bytes().to_vec()))
        },
    )?;
    let pipe = pipe()?;
    write(pipe.1, &res)?;
    Ok(())
}

pub fn program_entry() -> i8 {
    match program_entry_wrap() {
        Ok(_) => 0,
        Err(err) => err as i8,
    }
}
