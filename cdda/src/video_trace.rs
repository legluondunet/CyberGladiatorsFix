// SPDX-License-Identifier: GPL-3.0-or-later
//! Read-only MCI-video instrumentation. The original CYBER.EXE polls
//! mciSendStringA("status movie mode") after "play movie window", then
//! calls "close movie" / "close avivideo" before restoring graphics.
//! Wine can hang either in the polling loop or during the display transition.
//! Log the exact boundary without altering any return value or MCI command.
use std::sync::{Mutex,OnceLock};
use std::time::Instant;

#[derive(Default)]
struct MovieState {
    active:bool,
    polls:u64,
    last_mode:String,
    started:Option<Instant>,
}
static STATE:OnceLock<Mutex<MovieState>>=OnceLock::new();
fn state()->&'static Mutex<MovieState>{
    STATE.get_or_init(||Mutex::new(MovieState::default()))
}

fn command_is_video(command:&str)->bool{
    let trimmed=command.trim().to_ascii_lowercase();
    trimmed.split_ascii_whitespace().any(|word|word=="movie"||word=="avivideo")
}

pub fn before(command:&str)->Option<String>{
    if !command_is_video(command){return None;}
    let cmd=command.trim().to_ascii_lowercase();
    if cmd=="status movie mode"{return None;} // avoid 100s of lines/second.
    Some(format!("MCI_VIDEO BEGIN {command:?}"))
}
pub fn after(command:&str,code:u32,reply:&str)->Option<String>{
    if !command_is_video(command){return None;}
    let mut shared=state().lock().unwrap_or_else(|e|e.into_inner());
    after_in(&mut shared,command,code,reply)
}
fn after_in(state:&mut MovieState,command:&str,code:u32,reply:&str)->Option<String>{
    if !command_is_video(command){return None;}
    let cmd=command.trim().to_ascii_lowercase();
    if cmd=="play movie window" {
        state.active=code==0;
        state.started=if code==0{Some(Instant::now())}else{None};
        state.polls=0;
        state.last_mode.clear();
    }
    if cmd=="status movie mode"{
        state.polls=state.polls.saturating_add(1);
        let reply=reply.trim();
        let changed=state.last_mode!=reply;
        if changed{state.last_mode=reply.to_string();}
        // The 40-second SPLASH4.AVI is played at ~15 fps. If Wine never
        // stops reporting "playing" this 400-poll heartbeat reveals it.
        if !changed && state.polls%400!=0{return None;}
        let elapsed=state.started.map(|t|t.elapsed().as_secs()).unwrap_or(0);
        return Some(format!(
            "MCI_VIDEO STATUS polls={} elapsed={}s result={} mode={reply:?}",
            state.polls,elapsed,code));
    }
    let elapsed=state.started.map(|t|t.elapsed().as_secs()).unwrap_or(0);
    if cmd=="close movie"||cmd=="close avivideo" {
        state.active=false;
    }
    Some(format!("MCI_VIDEO END {command:?} result={code} reply={reply:?} elapsed={elapsed}s"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_movie_mci_commands_are_traced(){
        assert!(before("open avivideo").is_some());
        assert!(before("play movie window").is_some());
        assert!(before("close movie").is_some());
        assert!(before("status movie mode").is_none());
        assert!(before("set c time format tmsf").is_none());
        assert!(after("status c mode",0,"playing").is_none());
    }
    #[test]
    fn logs_status_transitions_without_spamming_poll_loop(){
        let mut state=MovieState::default();
        let _=after_in(&mut state,"play movie window",0,"");
        assert!(after_in(&mut state,"status movie mode",0,"playing").unwrap().contains("playing"));
        for _ in 0..398{assert!(after_in(&mut state,"status movie mode",0,"playing").is_none());}
        let m=after_in(&mut state,"status movie mode",0,"playing").unwrap();
        assert!(m.contains("polls=400"),"{m}");
        assert!(after_in(&mut state,"status movie mode",0,"stopped").unwrap().contains("stopped"));
        assert!(after_in(&mut state,"close movie",0,"").unwrap().contains("close movie"));
    }
    #[test]
    fn captures_failed_video_commands(){
        let message=after("play movie window",277,"").unwrap();
        assert!(message.contains("result=277"));
        assert!(after("close avivideo",0,"").unwrap().contains("close avivideo"));
    }
}
