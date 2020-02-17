use anyhow::Result;
use git2::{Config, Direction, Repository};

use crate::config;
use crate::config::ConfigValue;
use crate::simple_glob::{expand_refspec, ExpansionSide};

// given refspec for a remote: refs/heads/*:refs/remotes/origin
// master -> refs/remotes/origin/master
// refs/head/master -> refs/remotes/origin/master
pub fn get_fetch_remote_ref(
    repo: &Repository,
    config: &Config,
    branch: &str,
) -> Result<Option<String>> {
    let remote_name = config::get_remote(config, branch)?;
    get_remote_ref(repo, config, &remote_name, branch)
}

fn get_remote_ref(
    repo: &Repository,
    config: &Config,
    remote_name: &str,
    branch: &str,
) -> Result<Option<String>> {
    let remote = repo.find_remote(remote_name)?;
    let key = format!("branch.{}.merge", branch);
    let ref_on_remote: ConfigValue<String> =
        if let Some(ref_on_remote) = config::get(config, &key).read()? {
            ref_on_remote
        } else {
            return Ok(None);
        };
    assert!(
        ref_on_remote.starts_with("refs/"),
        "'git config branch.{}.merge' should start with 'refs/'",
        branch
    );

    if let Some(expanded) = expand_refspec(
        &remote,
        &ref_on_remote,
        Direction::Fetch,
        ExpansionSide::Right,
    )? {
        // TODO: is this necessary?
        let exists = repo.find_reference(&expanded).is_ok();
        if exists {
            Ok(Some(expanded))
        } else {
            Ok(None)
        }
    } else {
        Ok(None)
    }
}

// given refspec for a remote: refs/heads/*:refs/heads/*
// master -> refs/remotes/origin/master
// refs/head/master -> refs/remotes/origin/master
pub fn get_push_remote_ref(
    repo: &Repository,
    config: &Config,
    branch: &str,
) -> Result<Option<String>> {
    if let Some(RefOnRemote {
        remote_name,
        refname,
    }) = get_push_ref_on_remote(config, branch)?
    {
        if let Some(remote_ref) = get_remote_ref(repo, config, &remote_name, &refname)? {
            return Ok(Some(remote_ref));
        }
    }
    Ok(None)
}

#[derive(Eq, PartialEq, Clone)]
struct RefOnRemote {
    remote_name: String,
    refname: String,
}

fn get_push_ref_on_remote(config: &Config, branch: &str) -> Result<Option<RefOnRemote>> {
    let remote_name = config::get_push_remote(config, branch)?;
    // TODO: remote.<name>.push?

    if let Some(merge) =
        config::get(config, &format!("branch.{}.merge", branch)).parse_with(|ref_on_remote| {
            Ok(RefOnRemote {
                remote_name: remote_name.clone(),
                refname: ref_on_remote.to_string(),
            })
        })?
    {
        return Ok(Some(merge.clone()));
    }

    let push_default = config::get(config, "push.default")
        .with_default(&String::from("simple"))
        .read()?
        .expect("has default");

    match push_default.as_str() {
        "current" => Ok(Some(RefOnRemote {
            remote_name: remote_name.to_string(),
            refname: branch.to_string(),
        })),
        "upstream" | "tracking" | "simple" => {
            panic!("The current branch foo has no upstream branch.");
        }
        "nothing" | "matching" => {
            unimplemented!("push.default=nothing|matching is not implemented.")
        }
        _ => panic!("unexpected config push.default"),
    }
}