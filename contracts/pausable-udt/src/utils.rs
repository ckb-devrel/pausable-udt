use crate::error::Error;
use alloc::{ffi::CString, vec::Vec};
use ckb_ssri_sdk::public_module_traits::udt::UDT_LEN;
use ckb_std::{
    ckb_constants::Source,
    ckb_types::bytes::Bytes,
    debug,
    high_level::{decode_hex, load_cell_data, load_cell_lock_hash, QueryIter},
};

pub fn collect_inputs_amount() -> Result<u128, Error> {
    debug!("Entered collect_inputs_amount");
    let mut buf = [0u8; UDT_LEN];

    let udt_list = QueryIter::new(load_cell_data, Source::GroupInput)
        .map(|data| {
            if data.len() == UDT_LEN {
                buf.copy_from_slice(&data);
                Ok(u128::from_le_bytes(buf))
            } else {
                Err(Error::Encoding)
            }
        })
        .collect::<Result<Vec<_>, Error>>()?;
    Ok(udt_list.into_iter().sum::<u128>())
}

pub fn collect_outputs_amount() -> Result<u128, Error> {
    debug!("Entered collect_outputs_amount");
    let mut buf = [0u8; UDT_LEN];

    let udt_list = QueryIter::new(load_cell_data, Source::GroupOutput)
        .map(|data| {
            if data.len() == UDT_LEN {
                buf.copy_from_slice(&data);
                // u128 is 16 bytes
                Ok(u128::from_le_bytes(buf))
            } else {
                Err(Error::Encoding)
            }
        })
        .collect::<Result<Vec<_>, Error>>()?;
    Ok(udt_list.into_iter().sum::<u128>())
}

pub fn check_owner_mode(args: &Bytes) -> Result<bool, Error> {
    debug!("Entered check_owner_mode");
    let is_owner_mode = QueryIter::new(load_cell_lock_hash, Source::Input)
        .find(|lock_hash| args[..] == lock_hash[..])
        .is_some();
    debug!("Owner mode: {}", is_owner_mode);
    Ok(is_owner_mode)
}

pub fn format_pause_list(pause_list_str_vec: Vec<&str>) -> Vec<[u8; 32]> {
    let mut formatted_pause_u8_32_vec: Vec<[u8; 32]> = Vec::new();
    pause_list_str_vec.iter().for_each(|lock_hash_hex| {
        formatted_pause_u8_32_vec.push(
            decode_hex(CString::new(&lock_hash_hex[2..]).unwrap().as_c_str())
                .unwrap()
                .try_into()
                .unwrap(),
        );
    });
    formatted_pause_u8_32_vec
}

