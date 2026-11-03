// Adapted from ScorchedPlanetFix build.rs.
use std::process::Command;
fn main(){
    println!("cargo:rerun-if-changed=.git/HEAD");
    let sha=std::env::var("GITHUB_SHA").ok().filter(|v|!v.is_empty()).or_else(||{
        let o=Command::new("git").args(["rev-parse","--short=8","HEAD"]).output().ok()?;
        o.status.success().then(||String::from_utf8_lossy(&o.stdout).trim().to_string())
    }).unwrap_or_else(||"unknown".into());
    let short:String=sha.chars().filter(|c|c.is_ascii_hexdigit()).take(8).collect();
    println!("cargo:rustc-env=CYBER_COMMIT={}",if short.is_empty(){"unknown"}else{&short});
}
