use std::collections::HashMap;
use std::process::Command;

use color_eyre::Result;
use log::*;
use nix::unistd::{Gid, Uid};
use regex::Regex;

pub fn map_uids<I: Into<i32>>(pid: I, uids: &mut HashMap<Uid, Uid>) -> Result<()> {
    let pid = pid.into();
    let mut args = vec![pid.to_string()];
    for (old_uid, new_uid) in uids.iter() {
        args.push(old_uid.to_string());
        args.push(new_uid.to_string());
        args.push("1".to_string());
    }

    let newuidmap = Command::new("newuidmap").args(args).output();

    if newuidmap.is_err() {
        return newuidmap.map(|_| ()).map_err(|e| e.into());
    }

    let newuidmap = newuidmap?;
    let stderr = String::from_utf8(newuidmap.stderr)?;
    if let Some(bad_uid) = check_mapping_regex(r"newuidmap: uid range \[(\d+)-.*", &stderr)? {
        // Remove bad uid, continue to call newuidmap until it works
        uids.remove(&Uid::from_raw(bad_uid));
        return map_uids(pid, uids);
    }

    debug!("mapped uids {:#?}", uids);

    Ok(())
}

pub fn map_gids<I: Into<i32>>(pid: I, gids: &mut HashMap<Gid, Gid>) -> Result<()> {
    let pid = pid.into();
    let mut args = vec![pid.to_string()];
    for (old_gid, new_gid) in gids.iter() {
        args.push(old_gid.to_string());
        args.push(new_gid.to_string());
        args.push("1".to_string());
    }

    let newgidmap = Command::new("newgidmap").args(args).output();

    if newgidmap.is_err() {
        return newgidmap.map(|_| ()).map_err(|e| e.into());
    }

    let newgidmap = newgidmap?;
    let stderr = String::from_utf8(newgidmap.stderr)?;
    if let Some(bad_gid) = check_mapping_regex(r"newgidmap: gid range \[(\d+)-.*", &stderr)? {
        // Remove bad gid, continue to call newgidmap until it works
        gids.remove(&Gid::from_raw(bad_gid));
        return map_gids(pid, gids);
    }

    debug!("mapped gids {:#?}", gids);

    Ok(())
}

fn check_mapping_regex(regex: &str, stderr: &str) -> Result<Option<u32>> {
    let regex = Regex::new(regex)?;
    let bad_id = regex.captures(stderr);
    if let Some(bad_id) = bad_id {
        // Remove bad id, continue to call newuidmap until it works
        let bad_id = bad_id.get(1).unwrap().as_str().parse::<u32>().unwrap();
        Ok(Some(bad_id))
    } else {
        Ok(None)
    }
}
