use crate::{
    error::Error,
    modules::PausableUDT,
};

use alloc::vec;
use alloc::vec::Vec;
use ckb_std::{
    ckb_constants::Source, debug, high_level::load_cell_lock_hash
};

use ckb_ssri_sdk::public_module_traits::udt::{UDTPausable, UDT};

pub fn fallback() -> Result<(), Error> {
    debug!("Entered fallback");
    let mut lock_hashes: Vec<[u8; 32]> = vec![];

    let mut index = 0;
    while let Ok(lock_hash) = load_cell_lock_hash(index, Source::Input) {
        lock_hashes.push(lock_hash);
        index += 1;
    }

    index = 0;
    while let Ok(lock_hash) = load_cell_lock_hash(index, Source::Output) {
        lock_hashes.push(lock_hash);
        index += 1;
    }

    if PausableUDT::is_paused(&lock_hashes)?.iter().any(|&b| b) {
        return Err(Error::AbortedFromPause);
    }

    match PausableUDT::verify_mint() {
        Ok(_) => Ok(()),
        Err(_) => match PausableUDT::verify_transfer() {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        },
    }
}
