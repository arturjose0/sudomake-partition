//! Biblioteca do macread: leitura de discos Mac (APFS, HFS+), Linux (ext2/3/4, LVM) e imagens DMG no Windows.

pub mod apfs;
pub mod copy;
pub mod decmpfs;
pub mod device;
pub mod dmg;
pub mod dokan;
pub mod ext4;
pub mod fsservice;
pub mod fs;
pub mod hfsplus;
pub mod lvm;
pub mod lzvn;
pub mod open;
pub mod osdetect;
pub mod partition;
pub mod util;
