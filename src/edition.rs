// SPDX-License-Identifier: GPL-3.0-or-later
//! Edition identity for diagnostic discovery. A French fingerprint alone
//! never authorizes patching, installation or runtime memory-address hooks.
//! The validated US installer remains the sole writable production path.

/// SHA-256 of the original French PE32 file provided for static inspection.
/// Still requires authentication against the original CYBERG.BIN data track.
pub const FRENCH_CANDIDATE_SHA256:&str=
    "12db5e29120832ebc304695ecfa38f6c0d825afe2640433714d27f624da53cdb";
pub const FRENCH_CANDIDATE_BYTES:u64=352_288;

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum OriginalEdition{
    UsaRetail,
    FrenchCandidate,
    Unknown,
}
impl OriginalEdition{
    pub fn label(self)->&'static str{
        match self{
            Self::UsaRetail=>"US_RETAIL_VERIFIED",
            Self::FrenchCandidate=>"FR_CANDIDATE_NOT_AUTHENTICATED",
            Self::Unknown=>"UNKNOWN_REFUSE_PATCH",
        }
    }
    /// This gate deliberately remains closed to FR until all CD assets,
    /// edition-specific binary offsets and the native GameFrame hooks
    /// have been tested on the real French retail data.
    pub fn is_install_supported(self)->bool{
        matches!(self,Self::UsaRetail)
    }
}
pub fn from_original_exe(size:u64,sha256:&str)->OriginalEdition{
    if size==crate::patch::LEN as u64 && sha256==crate::patch::ORIGINAL {
        OriginalEdition::UsaRetail
    } else if size==FRENCH_CANDIDATE_BYTES && sha256==FRENCH_CANDIDATE_SHA256 {
        OriginalEdition::FrenchCandidate
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
        assert_eq!(from_original_exe(FRENCH_CANDIDATE_BYTES,FRENCH_CANDIDATE_SHA256),
            OriginalEdition::FrenchCandidate);
        assert_eq!(from_original_exe(FRENCH_CANDIDATE_BYTES+1,FRENCH_CANDIDATE_SHA256),
            OriginalEdition::Unknown);
        assert_eq!(from_original_exe(crate::patch::LEN as u64,FRENCH_CANDIDATE_SHA256),
            OriginalEdition::Unknown);
        assert_eq!(from_original_exe(FRENCH_CANDIDATE_BYTES,&"f".repeat(64)),
            OriginalEdition::Unknown);
    }
    #[test]
    fn french_and_unknown_never_unlock_the_us_patcher(){
        assert!(OriginalEdition::UsaRetail.is_install_supported());
        assert!(!OriginalEdition::FrenchCandidate.is_install_supported());
        assert!(!OriginalEdition::Unknown.is_install_supported());
    }
}
