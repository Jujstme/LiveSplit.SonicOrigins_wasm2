//extern crate alloc;
//use core::cell::RefCell;
//use alloc::{collections::BTreeMap, string::String, borrow::ToOwned};
use asr::{string::ArrayCString, Address, Address64, Process};

pub struct Rtti {
    base_address: Address,
    //cache: RefCell<BTreeMap<Address, String>>
}

impl Rtti {
    pub const fn new(base_address: Address) -> Self {
        Self {
            base_address,
            //cache: RefCell::new(BTreeMap::new()),
        }
    }

    pub fn lookup(&self, process: &Process, address: Address) -> Option<ArrayCString<128>> {
        //let mut cache = self.cache.borrow_mut();

        //if let Some(found_cached) = cache.get(&address) {
        //    Some(found_cached.to_owned())
        //} else {
        let base = process
            .read::<Address64>(address.add_signed(-0x8))
            .ok()?
            .add(0xC);

        let final_addr = self.base_address + process.read::<u32>(base).ok()? + 0x10 + 0x4;
        process.read(final_addr).ok()

        //let rtti_name = process.read::<ArrayCString<128>>(final_addr).ok()?;
        //let name = String::from_utf8_lossy(rtti_name.as_bytes());
        //let string = name.replace("@@", "").replace('@', "::");
        //cache.insert(address, string.clone());
        //Some(rtti_name)
        //}
    }
}
