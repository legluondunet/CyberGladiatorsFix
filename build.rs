// Adapted from ScorchedPlanetFix build.rs.
use std::process::Command;
fn main(){
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rustc-check-cfg=cfg(gameframe_embedded)");
    println!("cargo:rerun-if-env-changed=CYBER_GAMEFRAME_GLIDE_DLL_PATH");
    println!("cargo:rerun-if-env-changed=CYBER_GAMEFRAME_GLIDE_SHA256");
    if let Ok(path)=std::env::var("CYBER_GAMEFRAME_GLIDE_DLL_PATH"){
        let sha=std::env::var("CYBER_GAMEFRAME_GLIDE_SHA256")
            .expect("GameFrame x86 DLL path requires SHA-256 pin");
        assert!(std::path::Path::new(&path).is_file(),"Missing embedded GameFrame DLL");
        assert_eq!(sha.len(),64,"Invalid GameFrame SHA-256 pin");
        println!("cargo:rustc-cfg=gameframe_embedded");
    }
    // GitHub's GITHUB_SHA points to an ephemeral merge commit for pull_request
    // workflows. Prefer the commit physically checked out and compiled, so
    // --version and the user log identify the actual source revision.
    let sha=Command::new("git").args(["rev-parse","--short=8","HEAD"])
        .output().ok().and_then(|o|
            o.status.success().then(||String::from_utf8_lossy(&o.stdout).trim().to_string())
        ).filter(|s|!s.is_empty())
        .or_else(||std::env::var("GITHUB_SHA").ok().filter(|v|!v.is_empty()))
        .unwrap_or_else(||"unknown".into());
    let short:String=sha.chars().filter(|c|c.is_ascii_hexdigit()).take(8).collect();
    println!("cargo:rustc-env=CYBER_COMMIT={}",if short.is_empty(){"unknown"}else{&short});
}
