// SPDX-License-Identifier: GPL-3.0-or-later
//! Installer-rooted unified layout. No legacy layout migration.
use std::path::{Path,PathBuf};

pub const GAME:&str="game";
pub const MUSIC:&str="music";
pub const STATE:&str="state";
pub const DOCS:&str="docs";
pub const CONFIG:&str="CyberGladiatorsFix.ini";
pub const MANIFEST:&str="CyberGladiatorsFix.manifest";
pub const DLL_RECEIPT:&str="CyberGladiatorsFix.sha256";
pub const GLIDE_RECEIPT:&str="CyberGladiatorsFix.gameframe.sha256";

pub fn game(root:&Path)->PathBuf{root.join(GAME)}
pub fn music(root:&Path)->PathBuf{root.join(MUSIC)}
pub fn state(root:&Path)->PathBuf{root.join(STATE)}
pub fn docs(root:&Path)->PathBuf{root.join(DOCS)}
pub fn config(root:&Path)->PathBuf{root.join(CONFIG)}
pub fn manifest(root:&Path)->PathBuf{state(root).join(MANIFEST)}
pub fn dll_receipt(root:&Path)->PathBuf{state(root).join(DLL_RECEIPT)}
pub fn glide_receipt(root:&Path)->PathBuf{state(root).join(GLIDE_RECEIPT)}

#[cfg(test)]
mod tests{
    use super::*;
    #[test] fn paths_share_one_root(){
        let root=Path::new(r"C:\Cyber");
        assert_eq!(game(root),root.join("game"));
        assert_eq!(music(root),root.join("music"));
        assert_eq!(manifest(root),root.join("state").join(MANIFEST));
        assert_eq!(config(root),root.join(CONFIG));
    }
}
