// SPDX-License-Identifier: GPL-3.0-or-later
//! Edition identity for the two SHA-256-verified original retail CDs.
//! Installation also checks immutable edition-specific media fingerprints;
//! no unknown game binary or disc is allowed to unlock patching.

/// SHA-256 of CYBER/CYBER.EXE extracted from the authenticated CYBERG.BIN data track.
pub const FRENCH_ORIGINAL_SHA256:&str=
    "12db5e29120832ebc304695ecfa38f6c0d825afe2640433714d27f624da53cdb";
pub const FRENCH_ORIGINAL_BYTES:u64=352_288;

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum OriginalEdition{
    UsaRetail,
    FrenchRetail,
    Unknown,
}
impl OriginalEdition{
    pub fn label(self)->&'static str{
        match self{
            Self::UsaRetail=>"US_KNOWN_ORIGINAL_EXE",
            Self::FrenchRetail=>"FR_VERIFIED_ORIGINAL_EXE",
            Self::Unknown=>"UNKNOWN_REFUSE_PATCH",
        }
    }
    /// The installer independently checks original resource fingerprints
    /// before writing anything. Runtime integration still needs field tests.
    pub fn is_install_supported(self)->bool{
        matches!(self,Self::UsaRetail|Self::FrenchRetail)
    }
}
pub fn from_original_exe(size:u64,sha256:&str)->OriginalEdition{
    if size==crate::patch::LEN as u64 && sha256==crate::patch::ORIGINAL {
        OriginalEdition::UsaRetail
    } else if size==FRENCH_ORIGINAL_BYTES && sha256==FRENCH_ORIGINAL_SHA256 {
        OriginalEdition::FrenchRetail
    } else {
        OriginalEdition::Unknown
    }
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn exact_sha_and_size_are_required_for_both_editions(){
        assert_eq!(from_original_exe(crate::patch::LEN as u64,crate::patch::ORIGINAL),
            OriginalEdition::UsaRetail);
        assert_eq!(from_original_exe(FRENCH_ORIGINAL_BYTES,FRENCH_ORIGINAL_SHA256),
            OriginalEdition::FrenchRetail);
        assert_eq!(from_original_exe(FRENCH_ORIGINAL_BYTES+1,FRENCH_ORIGINAL_SHA256),
            OriginalEdition::Unknown);
        assert_eq!(from_original_exe(crate::patch::LEN as u64,FRENCH_ORIGINAL_SHA256),
            OriginalEdition::Unknown);
        assert_eq!(from_original_exe(FRENCH_ORIGINAL_BYTES,&"f".repeat(64)),
            OriginalEdition::Unknown);
    }
    #[test]
    fn verified_french_is_supported_but_unknown_is_rejected(){
        assert!(OriginalEdition::UsaRetail.is_install_supported());
        assert!(OriginalEdition::FrenchRetail.is_install_supported());
        assert!(!OriginalEdition::Unknown.is_install_supported());
    }
}
